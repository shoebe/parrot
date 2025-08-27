use parrot::client::Client;
use std::error::Error;

pub fn logger_init() {
    use std::io::Write;
    env_logger::builder()
        //.filter(None, log::LevelFilter::Trace)
        .filter_module("parrot", log::LevelFilter::Trace)
        .filter_module("songbird", log::LevelFilter::Info)
        //.filter_module("serenity", log::LevelFilter::Info)
        .format(|f, record| {
            let style = f.default_level_style(record.level());
            let time_style = f
                .default_level_style(log::Level::Info)
                .fg_color(Some(env_logger::fmt::style::Color::Rgb(
                    env_logger::fmt::style::RgbColor(100, 100, 100),
                )))
                .dimmed();
            writeln!(
                f,
                "{time_style}{}{time_style:#} {style}[{}] - {}{style:#} {time_style}{}:{}{time_style:#}",
                f.timestamp_micros(),
                record.level(),
                record.args(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
            )
        })
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    dotenv::dotenv().ok();

    logger_init();

    let mut parrot = Client::default().await?;
    if let Err(why) = parrot.start().await {
        log::error!("Fatality! Parrot crashed because: {why:?}");
    };

    Ok(())
}
