use std::fs;

pub mod day_one {
    use chrono::prelude::*;
    use lazy_static::lazy_static;
    use regex::Regex;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::io::Write;
    use std::process::{Command, Stdio};

    lazy_static! {
        static ref PHOTO_REGEX: Regex = Regex::new(r"\[\[dayone-moment://[^\]]+\]\]").unwrap();
        static ref MARKDOWN_PHOTO_REGEX: Regex =
            Regex::new(r"!\[\]\(dayone-moment://[^)]+\)").unwrap();
        static ref MARKDOWN_HEADING_REGEX: Regex = Regex::new(r"^#+\s").unwrap();
    }

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

    #[derive(Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Photo {
        pub md5: String,
        pub r#type: String,
        pub order_in_entry: u8,
    }

    impl Photo {
        pub fn link(&self) -> String {
            format!["[[./images/{}.{}]]", self.md5, self.r#type]
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
        pub photos: Option<Vec<Photo>>,
    }

    fn get_moon(moon: &str) -> String {
        match moon {
            "new" => "🌑",
            "waning-crescent" => "🌘",
            "last-quarter" => "🌗",
            "waning-gibbous" => "🌖",
            "full" => "🌕",
            "waxing-gibbous" => "🌔",
            "first-quarter" => "🌓",
            "waxing-crescent" => "🌒",
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

        pub fn title(&self, first_photo_link: Option<String>) -> Option<String> {
            if let Some(text) = &self.text {
                if let Some(line) = text.lines().next() {
                    let line = MARKDOWN_HEADING_REGEX.replace(&line, "").to_string();
                    let line = match first_photo_link {
                        Some(first_photo_link) => MARKDOWN_PHOTO_REGEX
                            .replace(&line, first_photo_link.as_str())
                            .to_string(),
                        None => line,
                    };
                    return Some(line.to_string());
                }
            }
            None
        }

        pub fn body(&self, photos: &Option<Vec<Photo>>) -> Option<String> {
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
                let mut body = panbody.lines().skip(4).collect::<Vec<&str>>().join("\n");
                if let Some(photos) = photos {
                    let mut photos: Vec<Photo> = photos.to_vec();
                    photos.sort_by_key(|p| p.order_in_entry);
                    for photo in photos {
                        body = PHOTO_REGEX
                            .replace(&body, photo.link().as_str())
                            .to_string();
                    }
                }
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
            let mut year_nums: Vec<_> = self.years.keys().collect();
            year_nums.sort();
            for y in year_nums {
                let year = self.years.get(y).unwrap();
                let mut month_nums: Vec<_> = year.months.keys().collect();
                println!("* {}", y);
                month_nums.sort();
                for m in month_nums {
                    let month = year.months.get(m).unwrap();
                    let mut day_nums: Vec<_> = month.days.keys().collect();
                    day_nums.sort();
                    println!("** {}-{} {}", y, m, Month::name_from(m));
                    for d in day_nums {
                        let day = month.days.get(d).unwrap();
                        println!("*** {}-{}-{} {}", y, m, d, Day::name_from(y, m, d));
                        for entry in &day.entries {
                            let first_photo = match &entry.photos {
                                Some(photos) => Some(photos[0].link()),
                                None => None,
                            };
                            println!(
                                "**** {}",
                                entry.title(first_photo).unwrap_or("Empty".to_string())
                            );
                            println!(":PROPERTIES:");
                            for (prop, value) in &entry.properties() {
                                println!(":{}: {}", prop, value);
                            }
                            println!(":END:");
                            println!("{}", entry.body(&entry.photos).unwrap_or("".to_string()));
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
