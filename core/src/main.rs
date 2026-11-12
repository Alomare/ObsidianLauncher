//! `obsidian`: the command line over obsidian-core. The commands live in [`obsidian_core::cli`], so
//! the app can run them too.

fn main() -> std::process::ExitCode {
    obsidian_core::cli::main()
}
