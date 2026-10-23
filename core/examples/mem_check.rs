//! Open every pack with ReadMode::Auto (same as the FFI path) and
//! report RSS, mirroring what the macOS app does at startup.

use rdict::{LookupEntry, RdictReader, ReadMode};

fn rss_mib() -> f64 {
    unsafe {
        let mut info: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT as libc::mach_msg_type_number_t;
        let kr = libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as libc::task_info_t,
            &mut count,
        );
        if kr == libc::KERN_SUCCESS {
            info.resident_size as f64 / 1024.0 / 1024.0
        } else {
            -1.0
        }
    }
}

fn main() {
    let packs = [
        "dictionary_en_en.rdict",
        "dictionary_en_zh.rdict",
        "dictionary_en_es.rdict",
        "dictionary_es_es.rdict",
        "dictionary_es_zh.rdict",
        "dictionary_es_en.rdict",
    ];

    println!("RSS before opens: {:.1} MiB", rss_mib());
    let mut readers = Vec::new();
    for p in packs {
        match RdictReader::open_with_mode(p, ReadMode::default()) {
            Ok(r) => {
                readers.push(r);
                println!("opened {p}: RSS {:.1} MiB", rss_mib());
            }
            Err(e) => eprintln!("FAILED to open {p}: {e}"),
        }
    }
    // Touch a lookup on the biggest pack so lazy decoding does real work.
    if let Some(r) = readers.first_mut() {
        let hit = match r.lookup("world") {
            Ok(Some(LookupEntry::Decoded(e))) => e.headword.clone(),
            Ok(Some(LookupEntry::Opaque { .. })) => "(opaque)".to_string(),
            _ => "(none)".to_string(),
        };
        println!("lookup 'world' in en_en: {hit}, RSS {:.1} MiB", rss_mib());
    }
    println!("RSS after all opens: {:.1} MiB", rss_mib());
    println!("open readers: {}", readers.len());
}
