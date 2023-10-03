use crate::ClanStorage;

pub struct ExcelStats {}

impl ExcelStats {
    pub fn new() -> Self {
        Self {}
    }

    pub fn populate_workbook(&self, stats: &ClanStorage) -> rust_xlsxwriter::Workbook {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();

        worksheet.set_name("Gold-Pass Tracking").unwrap();

        worksheet.write_string(0, 0, "Name");
        for idx in 1..8 {
            worksheet.write_string(0, idx, format!("CWL {}", idx));
        }
        for idx in stats.wars.iter().enumerate().map(|(i, _)| i) {
            worksheet.write_string(0, idx as u16 + 8, format!("War {}", idx));
        }
        for idx in 0..4 {
            worksheet.write_string(
                0,
                idx + 8 + stats.wars.len() as u16,
                format!("Raid {}", idx),
            );
        }

        let mut summaries: Vec<_> = stats
            .players_summary()
            .map(|(tag, sum)| (stats.player_names.get(&tag).unwrap(), tag, sum))
            .collect();
        summaries.sort_unstable_by(|(n, _, _), (n2, _, _)| n.cmp(n2));

        for (row, (name, tag, summary)) in
            summaries.into_iter().enumerate().map(|(c, d)| (c + 1, d))
        {
            let row = row as u32;

            worksheet.write_string(row, 0, name).unwrap();

            for w_index in 0..7 {
                let stars = match stats.cwl.wars.get(w_index) {
                    Some(war) => war
                        .members
                        .get(&tag)
                        .map(|s| s.attacks.iter().map(|a| a.stars).sum::<usize>())
                        .unwrap_or(0),
                    None => 0,
                };

                worksheet.write_number(row, (w_index + 1) as u16, stars as f64);
            }

            // TODO
            // Make sure this is actually sorted and not a random order
            for (w_index, (_, w_stats)) in stats.wars.iter().enumerate() {
                // TODO
                let stars = w_stats
                    .members
                    .get(&tag)
                    .map(|w| w.attacks.iter().map(|a| a.stars).sum())
                    .unwrap_or(0);

                worksheet.write_number(row, (w_index + 8) as u16, stars as f64);
            }

            // TODO
            // Make sure this is actually sorted by time
            for (r_index, (d, raid)) in stats.raid_weekend.iter().enumerate() {
                let loot = raid.members.get(&tag).map(|m| m.looted).unwrap_or(0);

                worksheet.write_number(row, (r_index + 8 + stats.wars.len()) as u16, loot as f64);
            }

            let score = (summary.war_stars as f64 / 66.0) * 100.0
                + (summary.cwl_stars as f64 / 21.0) * 100.0
                + (summary.raid_loot as f64 / 120000.0) * 100.0
                + (summary.games_score as f64 / 5000.0) * 100.0;
            worksheet.write_number(row, (8 + stats.wars.len() + 4) as u16, score);
        }

        workbook
    }
}
