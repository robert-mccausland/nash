use std::fmt::Display;

pub fn fmt_collection<I: Iterator<Item = F>, F: Display>(
    header: &str,
    separator: &str,
    footer: &str,
    data: I,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    f.write_str(header)?;
    let mut first = true;
    for element in data {
        if !first {
            f.write_str(separator)?;
        } else {
            first = false;
        }
        element.fmt(f)?;
    }
    f.write_str(footer)?;

    Ok(())
}
