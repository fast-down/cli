use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[inline]
pub async fn confirm(yes: bool, prompt: &str, default: bool) -> io::Result<bool> {
    fn get_text(value: bool) -> u8 {
        match value {
            true => b'Y',
            false => b'N',
        }
    }
    let text = match default {
        true => b"(Y/n) ",
        false => b"(y/N) ",
    };
    let mut stderr = io::stderr();
    stderr.write_all(prompt.as_bytes()).await?;
    stderr.write_all(text).await?;
    if yes {
        stderr.write_all(&[get_text(true), b'\n']).await?;
        return Ok(true);
    }
    stderr.flush().await?;
    loop {
        let mut input = String::with_capacity(4);
        BufReader::new(io::stdin()).read_line(&mut input).await?;
        break match input.trim() {
            "y" | "Y" => Ok(true),
            "n" | "N" => Ok(false),
            "" => Ok(default),
            _ => {
                stderr.write_all(prompt.as_bytes()).await?;
                stderr.write_all(text).await?;
                stderr.flush().await?;
                continue;
            }
        };
    }
}
