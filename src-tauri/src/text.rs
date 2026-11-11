//! Preparación del texto de la IA para leerlo en voz alta.

/// Elimina markdown y símbolos que la voz leería literalmente.
pub fn clean_for_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let l = l.trim_start_matches(|c: char| c == '#' || c == '>' || c == '-' || c == '*' || c == '•').trim();
        if l.is_empty() {
            continue;
        }
        if !out.is_empty() {
            // Une líneas como frases separadas.
            if !out.ends_with(['.', '!', '?', ':', ';', ',']) {
                out.push('.');
            }
            out.push(' ');
        }
        out.push_str(l);
    }
    // Enlaces markdown [texto](url) -> texto
    let mut cleaned = String::with_capacity(out.len());
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close) = chars[i..].iter().position(|&c| c == ']').map(|p| p + i) {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(end) = chars[close..].iter().position(|&c| c == ')').map(|p| p + close) {
                        cleaned.extend(&chars[i + 1..close]);
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        cleaned.push(chars[i]);
        i += 1;
    }
    cleaned
        .replace(['*', '_', '`', '#', '|'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Divide en fragmentos para sintetizar y reproducir en cadena:
/// el primero es corto para empezar a hablar cuanto antes.
pub fn speech_chunks(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end = matches!(c, '.' | '!' | '?' | ';' | ':')
            && chars.get(i + 1).map_or(true, |n| n.is_whitespace());
        if end {
            let s = cur.trim().to_string();
            if !s.is_empty() {
                sentences.push(s);
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        sentences.push(cur.trim().to_string());
    }

    let mut chunks: Vec<String> = Vec::new();
    for s in sentences {
        let limit = if chunks.len() == 1 { 120 } else { 280 };
        if let Some(last) = chunks.last_mut() {
            if last.chars().count() + 1 + s.chars().count() <= limit {
                last.push(' ');
                last.push_str(&s);
                continue;
            }
        }
        chunks.push(s);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown() {
        let t = "# Título\n**Hola**, mira [esto](https://x.com).\n- uno\n- dos\n```\ncode\n```";
        assert_eq!(clean_for_speech(t), "Título. Hola, mira esto. uno. dos");
    }

    #[test]
    fn keeps_decimals_together() {
        let c = speech_chunks("El valor es 3.14 aproximadamente. Otra frase.");
        assert_eq!(c.join(" "), "El valor es 3.14 aproximadamente. Otra frase.");
        assert!(c[0].starts_with("El valor es 3.14"));
    }

    #[test]
    fn first_chunk_is_short() {
        let long = "Primera frase corta. ".to_string() + &"Esta es una frase de relleno bastante larga. ".repeat(10);
        let c = speech_chunks(&long);
        assert!(c.len() > 1);
        assert!(c[0].chars().count() <= 120);
        assert!(c.iter().all(|x| x.chars().count() <= 330));
        assert_eq!(c.join(" ").split_whitespace().count(), long.split_whitespace().count());
    }

    #[test]
    fn empty() {
        assert!(speech_chunks("").is_empty());
    }
}
