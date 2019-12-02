use std::fs;

pub mod day_one {
    use chrono::prelude::*;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[derive(Deserialize)]
    pub struct Metadata {
        pub version: String,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Weather {
        pub conditions_description: Option<String>,
        pub moon_phase_code: Option<String>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Music {
        pub artist: String,
        pub track: String,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Location {
        pub longitude: f32,
        pub latitude: f32,
        pub place_name: String,
    }

    mod dates {
        use chrono::prelude::*;
        use serde::{Deserialize, Deserializer};
        const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%SZ";

        pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Utc.datetime_from_str(&s, FORMAT)
                .map_err(serde::de::Error::custom)
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Entry {
        #[serde(with = "dates")]
        pub creation_date: chrono::DateTime<chrono::Utc>,
        pub text: Option<String>,
        pub location: Option<Location>,
        pub weather: Option<Weather>,
        pub music: Option<Music>,
    }

    fn get_moon(moon: &str) -> String {
        match moon {
            "new" => "🌑",
            "full" => "🌕",
            "first-quarter" => "🌓",
            "last-quarter" => "🌗",
            "waning-crescent" => "🌘",
            "waxing-crescent" => "🌒",
            "waning-gibbous" => "🌖",
            "waxing-gibbous" => "🌔",
            _ => panic!("fake moon"),
        }
        .to_string()
    }

    impl Entry {
        pub fn year(&self) -> i32 {
            self.creation_date.year()
        }

        pub fn month(&self) -> u32 {
            self.creation_date.month()
        }

        pub fn day(&self) -> u32 {
            self.creation_date.day()
        }

        pub fn properties(&self) -> HashMap<String, String> {
            let mut props = HashMap::default();

            if let Some(weather) = &self.weather {
                if let Some(moon) = &weather.moon_phase_code {
                    props.insert("Moon".to_string(), get_moon(&moon));
                }
                if let Some(conditions) = &weather.conditions_description {
                    props.insert("Weather".to_string(), conditions.to_string());
                }
            }

            if let Some(music) = &self.music {
                props.insert(
                    "Music".to_string(),
                    format!("{} — {}", music.artist, music.track),
                );
            }

            if let Some(location) = &self.location {
                props.insert("Latitude".to_string(), format!("{}", location.latitude));
                props.insert("Longitude".to_string(), format!("{}", location.longitude));
                props.insert("Location".to_string(), location.place_name.to_string());
            }

            props
        }

        pub fn title(&self) -> Option<String> {
            if let Some(text) = &self.text {
                if let Some(line) = text.lines().next() {
                    let line = line.replace("### ", "");
                    let line = line.replace("# ", "");
                    return Some(line.to_string());
                }
            }
            None
        }

        pub fn body(&self) -> Option<String> {
            if let Some(text) = &self.text {
                let mut pandoc = Command::new("pandoc")
                    .args(&["-f", "markdown", "-t", "org", "--shift-heading-level-by=4"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("panda dog couldn't do it");
                {
                    let stdin = pandoc
                        .stdin
                        .as_mut()
                        .expect("couldn't open stdin for panda dog");
                    stdin
                        .write_all(text.as_bytes())
                        .expect("couldn't feed the panda dog");
                }

                let out = pandoc.wait_with_output().expect("Failed to read stdout");
                let panbody = String::from_utf8_lossy(&out.stdout).to_string();
                let body = panbody.lines().skip(4).collect::<Vec<&str>>().join("\n");
                return Some(body);
            }
            None
        }
    }

    #[derive(Deserialize)]
    pub struct Journal {
        pub metadata: Metadata,
        pub entries: Vec<Entry>,
    }
}

pub mod time_tree {
    use chrono::prelude::*;
    use std::collections::HashMap;

    struct Day {
        entries: Vec<crate::day_one::Entry>,
    }

    impl Day {
        fn name_from(y: &i32, m: &u32, d: &u32) -> String {
            Utc.ymd(*y, *m, *d).format("%A").to_string()
        }
    }

    struct Month {
        days: HashMap<u32, Day>,
    }

    impl Month {
        fn name_from(m: &u32) -> &'static str {
            match m {
                1 => "January",
                2 => "February",
                3 => "March",
                4 => "April",
                5 => "May",
                6 => "June",
                7 => "July",
                8 => "August",
                9 => "September",
                10 => "October",
                11 => "November",
                12 => "December",
                _ => panic!("Bad month"),
            }
        }
    }

    struct Year {
        months: HashMap<u32, Month>,
    }

    pub struct Root {
        years: HashMap<i32, Year>,
    }

    impl Root {
        fn add_entry(&mut self, entry: crate::day_one::Entry) {
            let year = self.years.entry(entry.year()).or_insert(Year {
                months: HashMap::default(),
            });

            let month = year.months.entry(entry.month()).or_insert(Month {
                days: HashMap::default(),
            });

            let day = month.days.entry(entry.day()).or_insert(Day {
                entries: Vec::default(),
            });

            day.entries.push(entry);
        }

        pub fn from(json: crate::day_one::Journal) -> Root {
            let mut journal = Root {
                years: HashMap::default(),
            };
            for entry in json.entries {
                journal.add_entry(entry);
            }
            journal
        }

        pub fn print(&self) {
            for (y, year) in &self.years {
                println!("* {}", y);
                for (m, month) in &year.months {
                    println!("** {}-{} {}", y, m, Month::name_from(m));
                    for (d, day) in &month.days {
                        println!("*** {}-{}-{} {}", y, m, d, Day::name_from(y, m, d));
                        for entry in &day.entries {
                            println!("**** {}", entry.title().unwrap_or("Empty".to_string()));
                            println!(":PROPERTIES:");
                            for (prop, value) in &entry.properties() {
                                println!(":{}: {}", prop, value);
                            }
                            println!(":END:");
                            println!("{}", entry.body().unwrap_or("".to_string()));
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let reader = fs::read("./Journal.json").expect("where is Journal.json?");
    let json: day_one::Journal = serde_json::from_slice(&reader).expect("couldn't unwrap");
    let journal = time_tree::Root::from(json);
    journal.print()
}
