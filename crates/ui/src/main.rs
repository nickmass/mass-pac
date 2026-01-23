use std::fs::File;
use std::path::PathBuf;

mod app;
mod audio;
mod filters;
mod gfx;
mod input;
mod runner;

use app::App;
use audio::{Audio, AudioDevices, Null, PipewireAudio, SamplesSender};
use filters::{CrtFilter, Filter, PixelatedFilter};
use gfx::GliumContext;
use runner::Runner;

fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();

    let mut no_filter = false;
    for arg in args.iter() {
        match arg.as_str() {
            "--no-filter" => no_filter = true,
            "--help" | "-h" => {
                usage();
                std::process::exit(0);
            }
            _ => (),
        }
    }

    args.retain(|arg| !arg.starts_with('-'));

    let rom = match load_rom(args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error loading rom:");
            eprintln!("{e:?}");
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let filter: Box<dyn Filter<GliumContext>> = if no_filter {
        Box::new(PixelatedFilter::new())
    } else {
        Box::new(CrtFilter::new())
    };

    let (audio, samples_tx) = init_audio();
    let sample_rate = audio.sample_rate();
    let mut app = App::new(filter, audio);
    let input = app.system_io();
    let back_buffer = app.back_buffer();

    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || {
            let runner = Runner::new(rom, input, back_buffer, samples_tx, sample_rate);

            runner.run()
        })
        .unwrap();

    app.run();
}

fn init_audio() -> (AudioDevices, SamplesSender) {
    match PipewireAudio::new() {
        _ if std::env::var("MASS_PAC_NO_AUDIO").is_ok() => {
            let (audio, tx) = Null::new();
            (audio.into(), tx)
        }
        Ok((audio, samples_tx)) => (audio.into(), samples_tx),
        Err(_) => {
            let (audio, tx) = Null::new();
            (audio.into(), tx)
        }
    }
}

fn load_rom(args: Vec<String>) -> Result<pacman::Rom, Box<dyn std::error::Error>> {
    let rom_path: PathBuf = args
        .first()
        .ok_or("supply path to pacman.zip or mspacman.zip")?
        .into();
    let container_name = rom_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("rom should have valid file name")?;

    let mut rom_builder = pacman::RomBuilder::new(container_name)?;
    let mut buf = Vec::new();

    let rom = if rom_path.is_dir() {
        for entry in rom_path.read_dir()?.take(16) {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };

            if entry.metadata()?.len() > 0x4000 {
                continue;
            }

            buf.clear();
            let mut file = File::open(entry.path())?;
            std::io::copy(&mut file, &mut buf)?;

            rom_builder.add_file(name, &buf);
        }
        rom_builder.build()?
    } else {
        let rom_file = File::open(rom_path)?;

        let mut zip = zip::ZipArchive::new(rom_file)?;
        for i in 0..zip.len().min(16) {
            let mut file = zip.by_index(i)?;
            if file.size() > 0x4000 {
                continue;
            }

            buf.clear();
            std::io::copy(&mut file, &mut buf)?;

            rom_builder.add_file(file.name(), &buf);
        }
        rom_builder.build()?
    };

    Ok(rom)
}

fn usage() {
    let usage = "Usage: mass-pac [OPTIONS] <ROM>
        
    Arguments:
    
        <ROM>
            Path to zip file or directory containing the rom
        
    Options:
    
        --no-filter
            Disable CRT filter
            
        --help  -h
            Display this message
            
    Examples:
    
        mass-pac roms/pacman.zip
    ";

    println!("{usage}");
}
