use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::net::IpAddr;

/// Parse nmap XML output file and extract IP addresses from hosthints
pub fn parse_nmap_xml(path: &str) -> Result<Vec<IpAddr>> {
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read XML file: {}", path))?;

    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut ips = Vec::new();
    let mut buf = Vec::new();
    let mut in_hosthint = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"hosthint" => {
                in_hosthint = true;
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"hosthint" => {
                in_hosthint = false;
            }
            Ok(Event::Empty(e)) if e.name().as_ref() == b"address" && in_hosthint => {
                // Extract addr attribute
                for attr in e.attributes() {
                    if let Ok(attr) = attr {
                        if attr.key.as_ref() == b"addr" {
                            if let Ok(addr_str) = std::str::from_utf8(&attr.value) {
                                if let Ok(ip) = addr_str.parse::<IpAddr>() {
                                    ips.push(ip);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow::anyhow!("XML parse error at position {}: {}",
                    reader.buffer_position(), e));
            }
            _ => {}
        }
        buf.clear();
    }

    if ips.is_empty() {
        return Err(anyhow::anyhow!("No IP addresses found in XML file"));
    }

    Ok(ips)
}
