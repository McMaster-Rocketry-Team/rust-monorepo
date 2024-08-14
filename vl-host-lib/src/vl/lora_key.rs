use rand::Rng;

pub fn gen_lora_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}

pub fn format_lora_key(key: &[u8; 32]) -> String {
    let mut formatted = String::new();
    formatted.push('[');
    for (i, byte) in key.iter().enumerate() {
        formatted.push_str(&format!("{}", byte));
        if i < key.len() - 1 {
            formatted.push(',');
            formatted.push(' ');
        }
    }
    formatted.push(']');
    formatted
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn print_lora_key() {
        let key = gen_lora_key();
        println!("{}", format_lora_key(&key));
    }
}
