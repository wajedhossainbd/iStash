//! Forecast the "Your system has run out of application memory" freeze.
//!
//! macOS rarely runs out of RAM as such: it compresses memory and then swaps to the startup disk. The
//! freeze comes when swap can no longer grow because the disk is (nearly) full. So the forecast is the
//! time until swap growth (plus other disk use, while memory is short) eats the free disk space, and it
//! only speaks up when memory is actually short.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Seconds (any epoch, increasing).
    pub t: f64,
    /// macOS's free-memory percentage (`kern.memorystatus_level`).
    pub free_pct: f64,
    pub swap_used: f64,
    /// Free bytes on the volume that holds swap.
    pub disk_free: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Watch,
    Warn,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Forecast {
    pub level: Level,
    /// Minutes until the freeze at the current trend; None when nothing is heading there.
    pub minutes_left: Option<f64>,
    pub reason: String,
}

// Calibration constants. shortcut: chosen from macOS behaviour reports, not measured freezes; recalibrate
// from recorded samples once the agent has logged real near-freezes.
/// Below this much free disk macOS cannot create more swap files.
pub const DISK_FLOOR: f64 = 3.0 * GB;
/// Free memory under this is "short": disk use then counts toward the freeze.
pub const LOW_FREE_PCT: f64 = 25.0;
/// Free memory under this is worth watching even without a trend.
pub const WATCH_FREE_PCT: f64 = 15.0;
/// Trends need at least this much history.
pub const MIN_SPAN_S: f64 = 60.0;
/// Only the last ten minutes count.
pub const WINDOW_S: f64 = 600.0;
const GB: f64 = 1e9;

/// Least-squares slope of y over t (per second).
fn slope(pts: &[(f64, f64)]) -> f64 {
    let n = pts.len() as f64;
    let (mt, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let (num, den) = pts.iter().fold((0.0, 0.0), |(a, b), (t, y)| (a + (t - mt) * (y - my), b + (t - mt).powi(2)));
    if den > 0.0 {
        num / den
    } else {
        0.0
    }
}

pub fn forecast(samples: &[Sample]) -> Forecast {
    let Some(last) = samples.last().copied() else {
        return Forecast { level: Level::Ok, minutes_left: None, reason: "No readings yet.".into() };
    };
    let short = last.free_pct < LOW_FREE_PCT;
    if short && last.disk_free <= DISK_FLOOR {
        return Forecast {
            level: Level::Critical,
            minutes_left: Some(0.0),
            reason: format!(
                "Memory is short ({:.0}% free) and the disk is almost full ({:.1} GB free): macOS cannot grow swap. Quit an app or free disk space now.",
                last.free_pct,
                last.disk_free / GB
            ),
        };
    }
    let recent: Vec<Sample> = samples.iter().copied().filter(|s| s.t >= last.t - WINDOW_S).collect();
    let span = last.t - recent[0].t;
    if recent.len() < 3 || span < MIN_SPAN_S {
        let level = if last.free_pct < WATCH_FREE_PCT { Level::Watch } else { Level::Ok };
        return Forecast {
            level,
            minutes_left: None,
            reason: format!("{:.0}% memory free; gathering a trend.", last.free_pct),
        };
    }
    let swap_rate = slope(&recent.iter().map(|s| (s.t, s.swap_used)).collect::<Vec<_>>());
    let disk_rate = -slope(&recent.iter().map(|s| (s.t, s.disk_free)).collect::<Vec<_>>());
    // While memory is fine, disk use (downloads, builds) is not heading for a memory freeze.
    let rate = if short { swap_rate.max(disk_rate) } else { swap_rate }; // bytes per second
    let per_min = rate * 60.0;
    if per_min < 0.05 * GB {
        let level = if last.free_pct < WATCH_FREE_PCT { Level::Watch } else { Level::Ok };
        return Forecast {
            level,
            minutes_left: None,
            reason: format!("{:.0}% memory free; swap is steady.", last.free_pct),
        };
    }
    let minutes = ((last.disk_free - DISK_FLOOR).max(0.0) / per_min).max(0.0);
    let level = if !short && last.free_pct >= WATCH_FREE_PCT {
        if minutes <= 30.0 {
            Level::Watch
        } else {
            Level::Ok
        }
    } else if minutes <= 10.0 {
        Level::Critical
    } else if minutes <= 30.0 {
        Level::Warn
    } else {
        Level::Watch
    };
    Forecast {
        level,
        minutes_left: Some(minutes),
        reason: format!(
            "Swap is growing {:.1} GB/min with {:.1} GB free on the disk and {:.0}% memory free: about {:.0} min until macOS runs out of application memory.",
            per_min / GB,
            last.disk_free / GB,
            last.free_pct,
            minutes
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample every 10 s for `n` steps with linear trends per minute.
    fn trace(n: usize, free: (f64, f64), swap: (f64, f64), disk: (f64, f64)) -> Vec<Sample> {
        (0..n)
            .map(|i| {
                let m = i as f64 * 10.0 / 60.0;
                Sample {
                    t: 1000.0 + i as f64 * 10.0,
                    free_pct: free.0 + free.1 * m,
                    swap_used: swap.0 + swap.1 * m,
                    disk_free: disk.0 + disk.1 * m,
                }
            })
            .collect()
    }

    #[test]
    fn this_mac_recorded_steady_state() {
        let csv = include_str!("../tests/fixtures/this-mac-steady.csv");
        let samples: Vec<Sample> = csv
            .lines()
            .skip(1)
            .map(|l| {
                let v: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
                Sample { t: v[0], free_pct: v[1], swap_used: v[2], disk_free: v[3] }
            })
            .collect();
        assert!(samples.len() >= 30);
        let f = forecast(&samples);
        assert_eq!((f.level, f.minutes_left), (Level::Ok, None), "{}", f.reason);
    }

    #[test]
    fn runaway_swap_on_a_small_disk_is_critical() {
        // 8% free, swap +1 GB/min, disk 10 GB and falling with it: (10 - 3) / 1 ≈ 7 min left at the end.
        let s = trace(30, (8.0, 0.0), (4.0 * GB, 1.0 * GB), (15.0 * GB, -GB));
        let f = forecast(&s);
        assert_eq!(f.level, Level::Critical, "{}", f.reason);
        let m = f.minutes_left.unwrap();
        assert!((6.0..8.0).contains(&m), "{m}");
    }

    #[test]
    fn slow_leak_is_a_warning_then_watch() {
        let s = trace(30, (12.0, -0.2), (2.0 * GB, 0.2 * GB), (8.0 * GB, -0.2 * GB));
        let f = forecast(&s);
        assert_eq!(f.level, Level::Warn, "{}", f.reason); // (7 - 3) / 0.2 = 20 min
        let s = trace(30, (12.0, 0.0), (2.0 * GB, 0.1 * GB), (40.0 * GB, -0.1 * GB));
        assert_eq!(forecast(&s).level, Level::Watch);
    }

    #[test]
    fn full_disk_with_short_memory_is_critical_now() {
        let f = forecast(&[Sample { t: 0.0, free_pct: 9.0, swap_used: 6.0 * GB, disk_free: 2.0 * GB }]);
        assert_eq!((f.level, f.minutes_left), (Level::Critical, Some(0.0)));
    }

    #[test]
    fn a_download_is_not_a_memory_problem() {
        let s = trace(30, (50.0, 0.0), (3.0 * GB, 0.0), (20.0 * GB, -2.0 * GB));
        let f = forecast(&s);
        assert_eq!((f.level, f.minutes_left), (Level::Ok, None), "{}", f.reason);
    }

    #[test]
    fn short_history_waits_for_a_trend() {
        let s = trace(3, (10.0, 0.0), (4.0 * GB, 5.0 * GB), (10.0 * GB, -5.0 * GB));
        let f = forecast(&s);
        assert_eq!((f.level, f.minutes_left), (Level::Watch, None));
        assert_eq!(forecast(&[]).level, Level::Ok);
    }

    #[test]
    fn old_samples_fall_out_of_the_window() {
        // A spike 20 minutes ago, flat since.
        let mut s = trace(5, (10.0, 0.0), (1.0 * GB, 3.0 * GB), (30.0 * GB, -3.0 * GB));
        let flat = trace(30, (10.0, 0.0), (4.0 * GB, 0.0), (27.0 * GB, 0.0));
        s.extend(flat.into_iter().map(|x| Sample { t: x.t + 1200.0, ..x }));
        assert_eq!(forecast(&s).minutes_left, None);
    }

    #[test]
    fn slopes() {
        assert!((slope(&[(0.0, 1.0), (1.0, 3.0), (2.0, 5.0)]) - 2.0).abs() < 1e-9);
        assert_eq!(slope(&[(1.0, 1.0), (1.0, 2.0)]), 0.0);
    }
}
