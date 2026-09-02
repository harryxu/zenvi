use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("=== Zenvi Icon & Asset Generator ===");

    let svg_path = "assets/zenvi-icon.svg";

    let svg_data = match fs::read(svg_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Could not read {}: {:?}", svg_path, e);
            return;
        }
    };

    let opt = usvg::Options::default();
    let tree = match usvg::Tree::from_data(&svg_data, &opt) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to parse SVG: {:?}", e);
            return;
        }
    };

    // 1. Setup packaging directories
    let macos_dir = Path::new("packaging/macos");
    let macos_iconset_dir = macos_dir.join("AppIcon.iconset");
    let linux_icons_dir = Path::new("packaging/linux/icons");

    fs::create_dir_all(&macos_iconset_dir).unwrap();
    fs::create_dir_all(&linux_icons_dir).unwrap();

    // macOS iconset sizes
    let macos_sizes: &[(u32, &str)] = &[
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
    ];

    for &(size, filename) in macos_sizes {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
        let scale = size as f32 / 1024.0;
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let out_path = macos_iconset_dir.join(filename);
        pixmap.save_png(&out_path).unwrap();
        println!("Generated (macOS): {}", out_path.display());
    }

    // Linux desktop icon sizes (standard hicolor sizes)
    let linux_sizes: &[u32] = &[16, 24, 32, 48, 64, 128, 256, 512, 1024];
    for &size in linux_sizes {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
        let scale = size as f32 / 1024.0;
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let out_path = linux_icons_dir.join(format!("zenvi_{size}x{size}.png"));
        pixmap.save_png(&out_path).unwrap();
        println!("Generated (Linux): {}", out_path.display());
    }

    // Copy scalable icon to linux
    let _ = fs::copy(svg_path, linux_icons_dir.join("zenvi.svg"));

    // 2. Compile into macOS AppIcon.icns (if on macOS or iconutil available)
    let icns_path = macos_dir.join("AppIcon.icns");
    let status = Command::new("iconutil")
        .args(&[
            "-c",
            "icns",
            macos_iconset_dir.to_str().unwrap(),
            "-o",
            icns_path.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Successfully generated macOS AppIcon: {}", icns_path.display());
        }
        Ok(s) => eprintln!("iconutil failed with exit code: {:?}", s),
        Err(e) => {
            // Non-macOS systems typically don't have iconutil
            println!("Skipping iconutil (not available on this platform): {:?}", e.kind());
        }
    }
}
