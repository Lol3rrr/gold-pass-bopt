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

        let mut column_index = (0..).into_iter();

        worksheet.write_string(0, column_index.next().unwrap(), "Name");
        for idx in 1..8 {
            worksheet.write_string(0, column_index.next().unwrap(), format!("CWL {}", idx));
        }
        worksheet.write_string(0, column_index.next().unwrap(), "CWL Score");
        for idx in stats.wars.iter().enumerate().map(|(i, _)| i) {
            worksheet.write_string(0, column_index.next().unwrap(), format!("War {}", idx + 1));
        }
        worksheet.write_string(0, column_index.next().unwrap(), "War Score");
        for idx in stats.raid_weekend.iter().enumerate().map(|(i, _)| i) {
            worksheet.write_string(0, column_index.next().unwrap(), format!("Raid {}", idx + 1));
        }
        worksheet.write_string(0, column_index.next().unwrap(), "Raid Score");

        worksheet.write_string(0, column_index.next().unwrap(), "Total Score");

        let mut summaries: Vec<_> = stats
            .players_summary()
            .map(|(tag, sum)| (stats.player_names.get(&tag).unwrap(), tag, sum))
            .collect();
        summaries.sort_unstable_by(|(n, _, _), (n2, _, _)| n.cmp(n2));

        for (row, (name, tag, summary)) in
            summaries.into_iter().enumerate().map(|(c, d)| (c + 1, d))
        {
            let row = row as u32;

            let mut column_index = (0..).into_iter();

            worksheet
                .write_string(row, column_index.next().unwrap(), name)
                .unwrap();

            for w_index in 0..7 {
                let stars = match stats.cwl.wars.get(w_index) {
                    Some(war) => war
                        .members
                        .get(&tag)
                        .map(|s| s.attacks.iter().map(|a| a.stars).sum::<usize>())
                        .unwrap_or(0),
                    None => 0,
                };

                worksheet.write_number(row, column_index.next().unwrap(), stars as f64);
            }
            worksheet.write_number(
                row,
                column_index.next().unwrap(),
                (summary.cwl_stars as f64 / 21.0) * 100.0,
            );

            // TODO
            // Make sure this is actually sorted and not a random order
            for (w_index, (_, w_stats)) in stats.wars.iter().enumerate() {
                // TODO
                let stars = w_stats
                    .members
                    .get(&tag)
                    .map(|w| w.attacks.iter().map(|a| a.stars).sum())
                    .unwrap_or(0);

                worksheet.write_number(row, column_index.next().unwrap(), stars as f64);
            }
            worksheet.write_number(
                row,
                column_index.next().unwrap(),
                (summary.war_stars as f64 / 66.0) * 100.0,
            );

            // TODO
            // Make sure this is actually sorted by time
            for (r_index, (d, raid)) in stats.raid_weekend.iter().enumerate() {
                let loot = raid.members.get(&tag).map(|m| m.looted).unwrap_or(0);

                worksheet.write_number(row, column_index.next().unwrap(), loot as f64);
            }
            worksheet.write_number(
                row,
                column_index.next().unwrap(),
                (summary.raid_loot as f64 / 120000.0) * 100.0,
            );

            let score = (summary.war_stars as f64 / 66.0) * 100.0
                + (summary.cwl_stars as f64 / 21.0) * 100.0
                + (summary.raid_loot as f64 / 120000.0) * 100.0
                + (summary.games_score as f64 / 5000.0) * 100.0;
            worksheet.write_number(row, column_index.next().unwrap(), score);
        }

        workbook
    }
}
