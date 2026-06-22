use crate::state_machine::SharedState;
use crate::util::MutexExt;
use crate::windows::{self, compute_hit_rect, get_pet_bounds, HitBox};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const TICK_INTERVAL_MS: u64 = 50;
const MOUSE_SLEEP_MS: u64 = 60_000;
const CIRCLE_TEMPLATE_POINTS: usize = 32;
const CIRCLE_TRACKING_WINDOW_MS: u64 = 2_500;
const CIRCLE_NEAR_RADIUS_MULTIPLIER: f64 = 1.8;
const CIRCLE_MIN_RADIUS_MULTIPLIER: f64 = 0.12;
const CIRCLE_MIN_POINTS: usize = 12;
const CIRCLE_MIN_BOUNDS: f64 = 24.0;
const CIRCLE_MATCH_THRESHOLD: f64 = 0.30;

/// Peek animation duration + margin — used to block re-peek during retraction.
const PEEK_ANIMATION_MS: u64 = 250;

/// Three-phase peek state machine to prevent oscillation.
/// Hidden → Peeking (mouse enters) → Retracting (mouse leaves) → Hidden.
/// While Retracting, new peek_in is blocked until the animation completes.
#[derive(Debug, Clone)]
pub enum PeekPhase {
    /// Pet is hidden at edge. Can transition to Peeking if mouse is near.
    Hidden,
    /// Pet is peeked out. Can transition to Retracting if mouse leaves.
    Peeking,
    /// Pet is animating back to hidden position. No new peek_in until done.
    Retracting(Instant),
}

#[derive(Debug, Clone)]
pub struct TickState {
    pub mouse_still_since: Instant,
    pub has_triggered_yawn: bool,
    pub has_triggered_wake: bool,
    pub last_eye_dx: f64,
    pub last_eye_dy: f64,
    pub peek_phase: PeekPhase,
    pub circle_points: Vec<CirclePoint>,
}

impl Default for TickState {
    fn default() -> Self {
        TickState {
            mouse_still_since: Instant::now(),
            has_triggered_yawn: false,
            has_triggered_wake: false,
            last_eye_dx: 0.0,
            last_eye_dy: 0.0,
            peek_phase: PeekPhase::Hidden,
            circle_points: Vec::new(),
        }
    }
}

pub type SharedTickState = Arc<Mutex<TickState>>;

#[derive(Clone, Serialize)]
struct EyeMovePayload {
    dx: f64,
    dy: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct CirclePoint {
    x: f64,
    y: f64,
    at: Instant,
}

type GesturePoint = (f64, f64);

fn path_length(points: &[GesturePoint]) -> f64 {
    points
        .windows(2)
        .map(|pair| {
            let dx = pair[1].0 - pair[0].0;
            let dy = pair[1].1 - pair[0].1;
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

fn resample_points(points: &[GesturePoint], target_count: usize) -> Option<Vec<GesturePoint>> {
    if points.len() < 2 || target_count < 2 {
        return None;
    }

    let total_len = path_length(points);
    if total_len <= f64::EPSILON {
        return None;
    }

    let interval = total_len / (target_count - 1) as f64;
    let mut sampled = Vec::with_capacity(target_count);
    sampled.push(points[0]);

    let mut prev = points[0];
    let mut distance_since_sample = 0.0;
    let mut index = 1;

    while index < points.len() && sampled.len() < target_count {
        let current = points[index];
        let dx = current.0 - prev.0;
        let dy = current.1 - prev.1;
        let segment_len = (dx * dx + dy * dy).sqrt();

        if segment_len <= f64::EPSILON {
            index += 1;
        } else if distance_since_sample + segment_len >= interval {
            let remain = interval - distance_since_sample;
            let ratio = remain / segment_len;
            let next = (prev.0 + dx * ratio, prev.1 + dy * ratio);
            sampled.push(next);
            prev = next;
            distance_since_sample = 0.0;
        } else {
            distance_since_sample += segment_len;
            prev = current;
            index += 1;
        }
    }

    while sampled.len() < target_count {
        sampled.push(*points.last()?);
    }

    Some(sampled)
}

fn normalize_points(points: &[GesturePoint]) -> Option<Vec<GesturePoint>> {
    if points.is_empty() {
        return None;
    }

    let (mut min_x, mut max_x) = (points[0].0, points[0].0);
    let (mut min_y, mut max_y) = (points[0].1, points[0].1);
    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let scale = width.max(height);
    if scale < CIRCLE_MIN_BOUNDS {
        return None;
    }

    let mut normalized: Vec<GesturePoint> = points
        .iter()
        .map(|&(x, y)| ((x - min_x) / scale, (y - min_y) / scale))
        .collect();
    let center_x = normalized.iter().map(|point| point.0).sum::<f64>() / normalized.len() as f64;
    let center_y = normalized.iter().map(|point| point.1).sum::<f64>() / normalized.len() as f64;
    for point in &mut normalized {
        point.0 -= center_x;
        point.1 -= center_y;
    }
    Some(normalized)
}

fn two_turn_template(clockwise: bool) -> Vec<GesturePoint> {
    (0..CIRCLE_TEMPLATE_POINTS)
        .map(|index| {
            let progress = index as f64 / (CIRCLE_TEMPLATE_POINTS - 1) as f64;
            let direction = if clockwise { -1.0 } else { 1.0 };
            let angle = direction * progress * std::f64::consts::TAU * 2.0;
            (angle.cos() * 0.5, angle.sin() * 0.5)
        })
        .collect()
}

fn best_loop_distance(points: &[GesturePoint], template: &[GesturePoint]) -> f64 {
    if points.is_empty() || points.len() != template.len() {
        return f64::INFINITY;
    }

    (0..template.len())
        .map(|shift| {
            points
                .iter()
                .enumerate()
                .map(|(index, point)| {
                    let target = template[(index + shift) % template.len()];
                    let dx = point.0 - target.0;
                    let dy = point.1 - target.1;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum::<f64>()
                / points.len() as f64
        })
        .fold(f64::INFINITY, f64::min)
}

fn is_two_turn_circle(points: &[CirclePoint]) -> bool {
    if points.len() < CIRCLE_MIN_POINTS {
        return false;
    }

    let raw_points: Vec<GesturePoint> = points.iter().map(|point| (point.x, point.y)).collect();
    let Some(sampled) = resample_points(&raw_points, CIRCLE_TEMPLATE_POINTS) else {
        return false;
    };
    let Some(normalized) = normalize_points(&sampled) else {
        return false;
    };

    let clockwise = two_turn_template(true);
    let counter_clockwise = two_turn_template(false);
    let distance = best_loop_distance(&normalized, &clockwise)
        .min(best_loop_distance(&normalized, &counter_clockwise));
    distance <= CIRCLE_MATCH_THRESHOLD
}

fn update_circle_progress(x: f64, y: f64, now: Instant, ts: &mut TickState) -> bool {
    ts.circle_points.push(CirclePoint { x, y, at: now });
    ts.circle_points.retain(|point| {
        now.duration_since(point.at) <= Duration::from_millis(CIRCLE_TRACKING_WINDOW_MS)
    });

    is_two_turn_circle(&ts.circle_points)
}

fn reset_circle_tracking(ts: &mut TickState) {
    ts.circle_points.clear();
}

/// Reads current_state directly from SharedState (lock hold time: one String clone).
/// This eliminates the need for a separate 200ms sync task + extra Arc<Mutex<String>>.
pub fn start_tick(app: AppHandle, state: SharedState) -> SharedTickState {
    let tick_state: SharedTickState = Arc::new(Mutex::new(TickState::default()));
    let tick_clone = tick_state.clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_INTERVAL_MS));
        let mut last_x: f64 = -1.0;
        let mut last_y: f64 = -1.0;

        loop {
            interval.tick().await;

            let cursor = match app.cursor_position() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let cx = cursor.x;
            let cy = cursor.y;

            let moved = (cx - last_x).abs() > 0.5 || (cy - last_y).abs() > 0.5;
            if moved {
                last_x = cx;
                last_y = cy;
            }

            let state_str = state.lock_or_recover().current_state.clone();
            let is_sleep_state = matches!(
                state_str.as_str(),
                "yawning" | "dozing" | "collapsing" | "sleeping"
            );

            // Single lock acquisition per tick for TickState
            let (should_yawn, should_wake) = {
                let mut ts = tick_clone.lock_or_recover();
                if moved {
                    ts.mouse_still_since = Instant::now();
                    ts.has_triggered_yawn = false;
                }
                // Reset wake flag when no longer in a sleep state
                if !is_sleep_state {
                    ts.has_triggered_wake = false;
                }
                let idle = ts.mouse_still_since.elapsed().as_millis() as u64;
                let yawn = state_str == "idle" && !ts.has_triggered_yawn && idle >= MOUSE_SLEEP_MS;
                let wake = moved && is_sleep_state && !ts.has_triggered_wake;
                if yawn {
                    ts.has_triggered_yawn = true;
                }
                if wake {
                    ts.has_triggered_wake = true;
                }
                (yawn, wake)
            };

            if should_wake {
                let _ = app.emit("trigger-wake", ());
            }
            if should_yawn {
                let _ = app.emit("trigger-yawn", ());
            }

            if let Some(bounds) = get_pet_bounds(&app) {
                let rect = compute_hit_rect(&bounds, &HitBox::DEFAULT);
                let center_x = (rect.left + rect.right) / 2.0;
                let center_y = (rect.top + rect.bottom) / 2.0;
                let dx = cx - center_x;
                let dy = cy - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                let size = bounds.width.max(bounds.height) as f64;
                let near_radius = size * CIRCLE_NEAR_RADIUS_MULTIPLIER;
                let min_radius = size * CIRCLE_MIN_RADIUS_MULTIPLIER;
                let now = Instant::now();

                let should_play_circle = {
                    let mut ts = tick_clone.lock_or_recover();
                    if distance >= min_radius && distance <= near_radius {
                        let complete = update_circle_progress(dx, dy, now, &mut ts);
                        if complete {
                            reset_circle_tracking(&mut ts);
                        }
                        complete
                    } else {
                        reset_circle_tracking(&mut ts);
                        false
                    }
                };

                if should_play_circle {
                    if let Some(pet) = app.get_webview_window("pet") {
                        let _ = pet.emit("play-spin-reaction", ());
                    }
                }
            }

            // Mini mode hover peek: three-phase state machine to prevent oscillation.
            // Hidden → Peeking → Retracting → Hidden.  While Retracting, peek_in
            // is blocked until the retraction animation finishes.
            {
                let is_mini = crate::prefs::is_mini_mode(&app);
                if is_mini {
                    if let Some(bounds) = get_pet_bounds(&app) {
                        // Use only the on-screen (visible) portion for near detection
                        // so the detection zone matches what the user actually sees.
                        let monitor = windows::monitor_for_bounds(&app, &bounds);
                        let (vis_x, vis_w) = if let Some(ref m) = monitor {
                            let left = bounds.x.max(m.x);
                            let right = (bounds.x + bounds.width as i32).min(m.x + m.width as i32);
                            (left, (right - left).max(0))
                        } else {
                            (bounds.x, bounds.width as i32)
                        };
                        let margin = (10.0 * windows::pet_scale_factor(&app)).round() as i32;
                        let near = cx >= (vis_x - margin) as f64
                            && cx <= (vis_x + vis_w + margin) as f64
                            && cy >= bounds.y as f64
                            && cy <= (bounds.y + bounds.height as i32) as f64;

                        // When suppressed (e.g. after mini entry), wait for mouse
                        // to leave the pet vicinity before allowing peek detection.
                        if crate::mini::is_peek_suppressed(&app) {
                            if !near {
                                crate::mini::clear_peek_suppression_if_mouse_away(&app);
                            }
                        } else {
                            let mut ts = tick_clone.lock_or_recover();
                            match ts.peek_phase {
                                PeekPhase::Hidden => {
                                    if near {
                                        ts.peek_phase = PeekPhase::Peeking;
                                        drop(ts);
                                        let _ = app.emit("mini-peek-in", ());
                                    }
                                }
                                PeekPhase::Peeking => {
                                    if !near {
                                        ts.peek_phase = PeekPhase::Retracting(Instant::now());
                                        drop(ts);
                                        let _ = app.emit("mini-peek-out", ());
                                    }
                                }
                                PeekPhase::Retracting(started) => {
                                    let done = started.elapsed()
                                        >= Duration::from_millis(PEEK_ANIMATION_MS);
                                    if done && !near {
                                        ts.peek_phase = PeekPhase::Hidden;
                                    } else if done && near {
                                        ts.peek_phase = PeekPhase::Peeking;
                                        drop(ts);
                                        let _ = app.emit("mini-peek-in", ());
                                    }
                                }
                            }
                        }
                    }
                } else {
                    tick_clone.lock_or_recover().peek_phase = PeekPhase::Hidden;
                    // Clear any stale suppression when leaving mini mode
                    crate::mini::clear_peek_suppression_if_mouse_away(&app);
                }
            }

            // Eye tracking: only in idle state
            if state_str == "idle" {
                if let Some(bounds) = get_pet_bounds(&app) {
                    let rect = compute_hit_rect(&bounds, &HitBox::DEFAULT);
                    let center_x = (rect.left + rect.right) / 2.0;
                    let center_y = (rect.top + rect.bottom) / 2.0;
                    // Convert physical-pixel deltas to logical so the
                    // directional normalization is DPI-independent.
                    let scale = windows::pet_scale_factor(&app);
                    let raw_dx = (cx - center_x) / scale;
                    let raw_dy = (cy - center_y) / scale;
                    let dist = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt().max(1.0);
                    let dx = (raw_dx / dist * 3.0).clamp(-3.0, 3.0);
                    let dy = (raw_dy / dist * 3.0).clamp(-3.0, 3.0);

                    let should_emit = {
                        let mut ts = tick_clone.lock_or_recover();
                        if (dx - ts.last_eye_dx).abs() > 0.1 || (dy - ts.last_eye_dy).abs() > 0.1 {
                            ts.last_eye_dx = dx;
                            ts.last_eye_dy = dy;
                            true
                        } else {
                            false
                        }
                    };
                    if should_emit {
                        if let Some(pet) = app.get_webview_window("pet") {
                            let _ = pet.emit("eye-move", EyeMovePayload { dx, dy });
                        }
                    }
                }
            }
        }
    });

    tick_state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle_points(points: &[GesturePoint], start: Instant) -> Vec<CirclePoint> {
        points
            .iter()
            .enumerate()
            .map(|(index, &(x, y))| CirclePoint {
                x,
                y,
                at: start + Duration::from_millis(index as u64 * 40),
            })
            .collect()
    }

    #[test]
    fn circle_template_matches_imperfect_two_turn_ellipse() {
        let now = Instant::now();
        let points: Vec<GesturePoint> = (0..60)
            .map(|index| {
                let progress = index as f64 / 59.0;
                let angle = 1.35 + progress * std::f64::consts::TAU * 2.0;
                let wobble = if index % 2 == 0 { 4.0 } else { -3.0 };
                (
                    12.0 + angle.cos() * 84.0 + wobble,
                    -8.0 + angle.sin() * 52.0,
                )
            })
            .collect();

        assert!(is_two_turn_circle(&circle_points(&points, now)));
    }

    #[test]
    fn circle_template_rejects_back_and_forth_motion() {
        let now = Instant::now();
        let points: Vec<GesturePoint> = (0..60)
            .map(|index| {
                let x = if index % 2 == 0 { -70.0 } else { 70.0 };
                let y = (index as f64 / 59.0 - 0.5) * 50.0;
                (x, y)
            })
            .collect();

        assert!(!is_two_turn_circle(&circle_points(&points, now)));
    }

    #[test]
    fn circle_tracking_discards_old_points() {
        let now = Instant::now();
        let mut ts = TickState::default();
        ts.circle_points.push(CirclePoint {
            x: 40.0,
            y: 0.0,
            at: now - Duration::from_millis(CIRCLE_TRACKING_WINDOW_MS + 1),
        });

        let _ = update_circle_progress(50.0, 0.0, now, &mut ts);

        assert_eq!(ts.circle_points.len(), 1);
    }
}
