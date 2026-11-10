//! Linha de comando: toda a entrada e saída fica aqui.
use flatmates::{flipbook, frames, raster, render, spec, Options, Output, EXAMPLES};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

const HELP: &str = "flatmates: ilustrações Corporate Memphis em SVG a partir de um JSON

USO
  flatmates render [cena.json] [-o saida.svg] [--png saida.png] [--scale 2] [--rig]
               [--seeds 1..8] [--size 1200x630]...
  flatmates animate [cena.json] [-o flip.svg] [--png-dir pasta] [--frames 12] [--fps 12]
  flatmates explain [cena.json]  onde cada item foi parar, defaults sorteados e avisos (JSON)
  flatmates example [nome]       imprime um exemplo (sem nome, lista os exemplos)
  flatmates catalog              lista tudo o que o JSON aceita

Sem arquivo de entrada, o JSON vem da entrada padrão.
Em `render`, sem -o e sem --png o SVG sai na saída padrão.
--seeds gera uma variação por semente e --size uma por tamanho; os arquivos
ganham sufixo (saida-s3.svg, saida-1080x1080.png).
--rig desenha o esqueleto por cima dos personagens.";

struct Args {
    input: Option<String>,
    out: Option<String>,
    png: Option<String>,
    png_dir: Option<String>,
    scale: f32,
    rig: bool,
    seeds: Vec<u64>,
    sizes: Vec<[u32; 2]>,
    frames: usize,
    fps: f64,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut a = Args {
        input: None,
        out: None,
        png: None,
        png_dir: None,
        scale: 1.0,
        rig: false,
        seeds: vec![],
        sizes: vec![],
        frames: 12,
        fps: 12.0,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| it.next().cloned().ok_or_else(|| format!("{flag} precisa de um valor"));
        match arg.as_str() {
            "-o" | "--out" => a.out = Some(value("-o")?),
            "--png" => a.png = Some(value("--png")?),
            "--png-dir" => a.png_dir = Some(value("--png-dir")?),
            "--scale" => {
                a.scale = value("--scale")?.parse().map_err(|_| "--scale precisa ser um número".to_string())?;
                if !(0.1..=8.0).contains(&a.scale) {
                    return Err("--scale vai de 0.1 a 8".into());
                }
            }
            "--seeds" => {
                let v = value("--seeds")?;
                let bad = || format!("--seeds '{v}' inválido; use 1..8 ou 3,7,9");
                a.seeds = match v.split_once("..") {
                    Some((lo, hi)) => {
                        let (lo, hi): (u64, u64) = (lo.parse().map_err(|_| bad())?, hi.parse().map_err(|_| bad())?);
                        if lo > hi || hi - lo > 63 {
                            return Err("--seeds aceita até 64 sementes em ordem crescente".into());
                        }
                        (lo..=hi).collect()
                    }
                    None => v.split(',').map(|s| s.trim().parse().map_err(|_| bad())).collect::<Result<_, _>>()?,
                };
            }
            "--size" => {
                let v = value("--size")?;
                let wh = v.split_once('x').and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]));
                let [w, h]: [u32; 2] = wh.ok_or_else(|| format!("--size '{v}' inválido; use 1200x630"))?;
                if !(64..=8192).contains(&w) || !(64..=8192).contains(&h) {
                    return Err("--size: largura e altura vão de 64 a 8192".into());
                }
                a.sizes.push([w, h]);
            }
            "--frames" => a.frames = value("--frames")?.parse().map_err(|_| "--frames precisa ser um inteiro".to_string())?,
            "--fps" => a.fps = value("--fps")?.parse().map_err(|_| "--fps precisa ser um número".to_string())?,
            "--rig" => a.rig = true,
            "-" => a.input = None,
            flag if flag.starts_with('-') => return Err(format!("opção '{flag}' não existe. Rode `flatmates help`.")),
            path => a.input = Some(path.to_string()),
        }
    }
    Ok(a)
}

fn read_input(input: &Option<String>) -> Result<String, String> {
    match input {
        Some(path) => std::fs::read_to_string(path).map_err(|e| format!("não consegui ler {path}: {e}")),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map_err(|e| format!("não consegui ler a entrada: {e}"))?;
            Ok(s)
        }
    }
}

fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("não consegui gravar {path}: {e}"))
}

/// `saida.svg` + `-s3` vira `saida-s3.svg`.
fn with_suffix(path: &str, suffix: &str) -> String {
    let p = Path::new(path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("saida");
    let name = match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}{suffix}.{ext}"),
        None => format!("{stem}{suffix}"),
    };
    p.with_file_name(name).to_string_lossy().into_owned()
}

fn report(out: &Output) {
    for w in &out.warnings {
        eprintln!("aviso: {w}");
    }
}

fn cmd_render(args: &[String]) -> Result<(), String> {
    let a = parse_args(args)?;
    let json = read_input(&a.input)?;
    let seeds: Vec<Option<u64>> = if a.seeds.is_empty() { vec![None] } else { a.seeds.iter().map(|s| Some(*s)).collect() };
    let sizes: Vec<Option<[u32; 2]>> = if a.sizes.is_empty() { vec![None] } else { a.sizes.iter().map(|s| Some(*s)).collect() };
    let many = seeds.len() * sizes.len() > 1;
    if many && a.out.is_none() && a.png.is_none() {
        return Err("com --seeds ou vários --size, diga onde gravar com -o ou --png".into());
    }
    for seed in &seeds {
        for size in &sizes {
            let opts = Options { rig: a.rig, time: 0.0, seed: *seed, size: *size };
            let result = render(&json, &opts)?;
            report(&result);
            let mut suffix = String::new();
            if let (Some(s), true) = (seed, seeds.len() > 1) {
                suffix.push_str(&format!("-s{s}"));
            }
            if let (Some([w, h]), true) = (size, sizes.len() > 1) {
                suffix.push_str(&format!("-{w}x{h}"));
            }
            if let Some(path) = &a.out {
                write_file(&with_suffix(path, &suffix), result.svg.as_bytes())?;
            }
            if let Some(path) = &a.png {
                write_file(&with_suffix(path, &suffix), &raster::png(&result.svg, a.scale)?)?;
            }
            if a.out.is_none() && a.png.is_none() {
                std::io::stdout().write_all(result.svg.as_bytes()).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

fn cmd_animate(args: &[String]) -> Result<(), String> {
    let a = parse_args(args)?;
    let json = read_input(&a.input)?;
    let opts = Options { rig: a.rig, ..Options::default() };
    if let Some(dir) = &a.png_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("não consegui criar {dir}: {e}"))?;
        let all = frames(&json, a.frames, &opts)?;
        report(&all[0]);
        for (k, f) in all.iter().enumerate() {
            let path = Path::new(dir).join(format!("frame-{k:03}.png"));
            write_file(&path.to_string_lossy(), &raster::png(&f.svg, a.scale)?)?;
        }
    }
    if a.out.is_some() || a.png_dir.is_none() {
        let book = flipbook(&json, a.frames, a.fps, &opts)?;
        if a.png_dir.is_none() {
            report(&book);
        }
        match &a.out {
            Some(path) => write_file(path, book.svg.as_bytes())?,
            None => std::io::stdout().write_all(book.svg.as_bytes()).map_err(|e| e.to_string())?,
        }
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let pretty = |v: &serde_json::Value| serde_json::to_string_pretty(v).map_err(|e| e.to_string());
    match args.first().map(String::as_str) {
        Some("render") => cmd_render(&args[1..]),
        Some("animate") => cmd_animate(&args[1..]),
        Some("explain") => {
            let a = parse_args(&args[1..])?;
            let out = render(&read_input(&a.input)?, &Options::default())?;
            println!("{}", pretty(&out.explain)?);
            Ok(())
        }
        Some("example") => match args.get(1) {
            None => {
                for (name, _) in EXAMPLES {
                    println!("{name}");
                }
                Ok(())
            }
            Some(name) => {
                let found = EXAMPLES.iter().find(|(n, _)| n == name);
                let (_, json) =
                    found.ok_or_else(|| format!("exemplo '{name}' não existe. Opções: {}", EXAMPLES.map(|e| e.0).join(", ")))?;
                print!("{json}");
                Ok(())
            }
        },
        Some("catalog") => {
            println!("{}", pretty(&spec::catalog())?);
            Ok(())
        }
        Some("help" | "--help" | "-h") | None => {
            println!("{HELP}");
            Ok(())
        }
        Some(other) => Err(format!("comando '{other}' não existe. Rode `flatmates help`.")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
