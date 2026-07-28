#![forbid(unsafe_code)]

//! The MINE ASCII banner. Solid block letters, printed at the start of
//! `mine setup`.

/// Prints the MINE banner to stdout.
pub fn print() {
    // 5-row solid block "MINE" rendered with full block U+2588.
    // Each letter is 5 columns wide; letters separated by one blank column.
    let art = "\
███╗   ███╗██╗███╗   ██╗███████╗
████╗ ████║██║████╗  ██║██╔════╝
██╔████╔██║██║██╔██╗ ██║█████╗
██║╚██╔╝██║██║██║╚██╗██║██╔══╝
██║ ╚═╝ ██║██║██║ ╚████║███████╗
╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚══════╝

  MINE Is Not Everyone's.
";
    println!("{art}");
}
