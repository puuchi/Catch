//! Closed intervals do not change between tracking revisions. Only open
//! intervals depend on now; timezone/day bounds are part of the cache key.
use catch_core::{
    growth::{self, DayBounds},
    tracking::{self, Document},
};
#[derive(Default)]
pub(crate) struct EffortCache(Option<Projection>);
struct Projection {
    revision: u64,
    day: String,
    start: i64,
    end: i64,
    closed: i64,
    open: Vec<i64>,
}
impl EffortCache {
    pub fn invested(
        &mut self,
        document: &Document,
        bounds: &DayBounds<'_>,
        now: i64,
    ) -> Result<i64, String> {
        let valid = self.0.as_ref().is_some_and(|p| {
            p.revision == document.revision
                && p.day == bounds.day
                && p.start == bounds.start
                && p.end == bounds.end
        });
        if !valid {
            // At the day's start every open interval contributes zero. Closed
            // intervals retain their normal clipped contribution, even when
            // historical edits include timestamps later than now.
            let closed = growth::invested(document, bounds, bounds.start)?;
            let open = document
                .activities
                .iter()
                .flat_map(|m| &m.segments)
                .filter(|s| s.end.is_none())
                .map(|s| tracking::timestamp(&s.start))
                .collect::<Result<Vec<_>, _>>()?;
            self.0 = Some(Projection {
                revision: document.revision,
                day: bounds.day.into(),
                start: bounds.start,
                end: bounds.end,
                closed,
                open,
            });
        }
        let projection = self.0.as_ref().unwrap();
        projection
            .open
            .iter()
            .try_fold(projection.closed, |sum, start| {
                let elapsed = now
                    .min(bounds.end)
                    .saturating_sub((*start).max(bounds.start))
                    .max(0);
                sum.checked_add(elapsed)
                    .ok_or_else(|| "累计时长超出范围".into())
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_projection_matches_reference_through_original_behavior_traces_and_day_changes() {
        let scenarios: serde_json::Value = serde_json::from_str(include_str!(
            "../../catch-core/tests/fixtures/tracking-parity.json"
        ))
        .unwrap();
        let mut cache = EffortCache::default();
        let mut nonzero = 0;
        for scenario in scenarios["scenarios"].as_array().unwrap() {
            // Independent profiles may reuse revisions: cache belongs to one
            // Service, so replacing the service must also replace its cache.
            cache.0 = None;
            for step in scenario["steps"].as_array().unwrap() {
                let document: Document = serde_json::from_value(step["expected"].clone()).unwrap();
                let now = step["now"].as_i64().unwrap();
                let today = now.div_euclid(86_400_000) * 86_400_000;
                for day_offset in [today - 86_400_000, today, today + 86_400_000] {
                    let bounds = DayBounds {
                        day: "2026-11-14",
                        start: day_offset,
                        end: day_offset + 86_400_000,
                    };
                    for at in [now, now.saturating_add(17_000), now.saturating_sub(30_000)] {
                        nonzero +=
                            usize::from(growth::invested(&document, &bounds, at).unwrap() > 0);
                        assert_eq!(
                            cache.invested(&document, &bounds, at),
                            growth::invested(&document, &bounds, at)
                        );
                    }
                }
            }
        }
        assert!(
            nonzero > 50,
            "reference traces must exercise nonzero invested time"
        );
    }
    #[test]
    #[ignore = "manual synthetic performance sample, not a timing assertion"]
    fn synthetic_history_performance_sample() {
        use std::{hint::black_box, time::Instant};
        let activities: Vec<_> = (0..1000).map(|index| {
            let start = tracking::iso(index * 2000).unwrap();
            let end = tracking::iso(index * 2000 + 1000).unwrap();
            serde_json::json!({"id":format!("synthetic-{index}"),"name":"合成历史","color":"#2679e8","startTime":start,"endTime":end,"status":"completed","segments":[{"start":start,"end":end}],"todoId":null,"createdAt":start,"updatedAt":end,"pauseReason":null})
        }).collect();
        let document: Document =
            serde_json::from_value(serde_json::json!({"version":1,"activities":activities}))
                .unwrap();
        let document = document.migrated().unwrap();
        let bounds = DayBounds {
            day: "1970-01-01",
            start: 0,
            end: 86_400_000,
        };
        let mut cache = EffortCache::default();
        assert_eq!(
            cache.invested(&document, &bounds, 3_000_000).unwrap(),
            1_000_000
        );
        let reference = Instant::now();
        for index in 0..200 {
            black_box(growth::invested(black_box(&document), &bounds, 3_000_000 + index).unwrap());
        }
        let reference_ns = reference.elapsed().as_nanos();
        let projected = Instant::now();
        for index in 0..200 {
            black_box(
                cache
                    .invested(black_box(&document), &bounds, 3_000_000 + index)
                    .unwrap(),
            );
        }
        eprintln!(
            "{}",
            serde_json::json!({"fixture":"1000 synthetic closed matters","iterations":200,"reference_ns":reference_ns,"cached_ns":projected.elapsed().as_nanos(),"scope":"warm effort projection only; not GUI latency or application memory"})
        );
    }
}
