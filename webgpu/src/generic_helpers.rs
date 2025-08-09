use crate::misc;

pub fn calculate_secs_elapsed(before: f32) -> f32 {
    let mut percent_update = (misc::now() - before)/ 1000f32;
    if percent_update > 1f32 {
        percent_update = 1f32;
    }
    percent_update
}

pub fn convert_str_u32_to_char(string: &str) -> Option<char> {
    match string.parse::<u32>() {
        Ok(x) => {
            char::from_u32(x)
        },
        Err(_) => None,
    }
}

pub fn vec_starts_with(string: &str, list: &Vec<String>) -> bool {
    list.iter().any(|prefix| string.starts_with(prefix))
}

pub fn round_f32(val: f32, places: u32) -> f32 {
    let unit = 10u32.pow(places) as f32;
    (val * unit).round()/ unit
}