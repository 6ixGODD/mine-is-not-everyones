#![forbid(unsafe_code)]

//! The MINE ASCII banner. Solid block letters, printed at the start of
//! `mine setup` (inline in non-TTY contexts, or at the top of the
//! alternate-screen selector in interactive contexts).

/// The banner art (ASCII block letters only; callers add the tagline and
/// spacing as needed).
pub const ASCII_ART: &str = "\
███╗   ███╗██╗███╗   ██╗███████╗
████╗ ████║██║████╗  ██║██╔════╝
██╔████╔██║██║██╔██╗ ██║█████╗
██║╚██╔╝██║██║██║╚██╗██║██╔══╝
██║ ╚═╝ ██║██║██║ ╚████║███████╗
╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚══════╝
";

/// Prints the MINE banner with tagline to stdout (used in non-TTY/inline
/// contexts).
pub fn print() {
    println!("{ASCII_ART}");
    println!("  MINE Is Not Everyone's.");
    println!();
}
