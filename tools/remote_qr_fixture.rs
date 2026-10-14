//! Emit actual production terminal QR cells for Android ZXing interoperability tests.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ratatui::style::Color;
use std::{io::Write, path::PathBuf};
use zenpi::remote::pairing::{PairingInvite, unix_seconds};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
    std::fs::create_dir_all(&output)?;
    let now = unix_seconds()?;
    let cases = [
        ("ipv4", "192.168.50.55:7777"),
        ("ipv6", "[fd00::1234]:65535"),
        ("ipv4-port-min", "10.255.255.254:1"),
        (
            "ipv6-long",
            "[fddd:ffff:ffff:ffff:ffff:ffff:ffff:ffff]:65535",
        ),
        ("ipv4-loopback", "127.0.0.1:65535"),
        ("ipv6-loopback", "[::1]:1"),
    ];
    let mut variants = cases
        .iter()
        .map(|(name, address)| (name.to_string(), *address, None))
        .collect::<Vec<_>>();
    for index in 0..48 {
        variants.push((
            format!("payload-{index:02}"),
            cases[index % cases.len()].1,
            Some(index),
        ));
    }
    for (name, address, variant) in &variants {
        let fixture_digest = variant.map(|index| {
            ring::digest::digest(
                &ring::digest::SHA256,
                format!("zenpi-qr-fixture-{index}").as_bytes(),
            )
        });
        let invite = PairingInvite {
            schema_version: 1,
            address: address.parse()?,
            fingerprint: fixture_digest.as_ref().map_or_else(
                || "a".repeat(64),
                |digest| {
                    digest
                        .as_ref()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect()
                },
            ),
            token: fixture_digest.as_ref().map_or_else(
                || URL_SAFE_NO_PAD.encode((0u8..32).collect::<Vec<_>>()),
                |digest| URL_SAFE_NO_PAD.encode(digest.as_ref()),
            ),
            expires_at: now + 300,
            hints: vec![zenpi::remote::pairing::AddressHint::lan(address.parse()?)],
        };
        let lines = zenpi::host_pair::qr_lines(&invite)?;
        let scale = 4;
        let width = lines[0].spans.len() * scale;
        let height = lines.len() * 2 * scale;
        let mut pixels = vec![255u8; width * height];
        for (row, line) in lines.iter().enumerate() {
            for (column, span) in line.spans.iter().enumerate() {
                for (half, color) in [span.style.fg, span.style.bg].into_iter().enumerate() {
                    for y in 0..scale {
                        for x in 0..scale {
                            pixels[((row * 2 + half) * scale + y) * width + column * scale + x] =
                                if color == Some(Color::Black) { 0 } else { 255 };
                        }
                    }
                }
            }
        }
        let mut image = std::fs::File::create(output.join(format!("terminal-qr-{name}.pgm")))?;
        write!(image, "P5\n{width} {height}\n255\n")?;
        image.write_all(&pixels)?;
        let fixture = serde_json::json!({"schema_version":1,"fixture_only":true,"now":now,"address":address,
            "uri":invite.to_uri()?,"width":width,"height":height});
        std::fs::write(
            output.join(format!("terminal-qr-{name}.json")),
            serde_json::to_vec_pretty(&fixture)?,
        )?;
    }
    std::fs::write(
        output.join("terminal-qr-manifest.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema_version":"stage2-terminal-qr-fixture/v1","fixture_only":true,
            "cases":variants.iter().map(|(name, _, _)| name).collect::<Vec<_>>()}),
        )?,
    )?;
    Ok(())
}
