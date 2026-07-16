//! Minimal FTP STOR client (matches Go utils.FTPUpload).

use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn ftp_upload(
    addr: &str,
    user: &str,
    pass: &str,
    remote_name: &str,
    mut reader: impl Read,
) -> Result<()> {
    let sock = addr
        .to_socket_addrs()
        .with_context(|| format!("resolve {addr}"))?
        .next()
        .ok_or_else(|| anyhow::anyhow!("resolve empty: {addr}"))?;
    let mut conn =
        TcpStream::connect_timeout(&sock, Duration::from_secs(30)).with_context(|| format!("ftp dial {addr}"))?;
    conn.set_read_timeout(Some(Duration::from_secs(300)))?;
    conn.set_write_timeout(Some(Duration::from_secs(300)))?;

    let mut br = BufReader::new(conn.try_clone()?);
    let _ = read_ftp(&mut br)?;
    ftp_cmd(&mut conn, &mut br, &format!("USER {user}"), "331")?;
    ftp_cmd(&mut conn, &mut br, &format!("PASS {pass}"), "230")?;
    write_ftp(&mut conn, "TYPE I")?;
    let _ = read_ftp(&mut br)?;
    write_ftp(&mut conn, "PASV")?;
    let pasv = read_ftp(&mut br)?;
    let (host, port) = parse_pasv(&pasv)?;
    let data_addr = format!("{host}:{port}");
    let data_sock = data_addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("pasv resolve empty"))?;
    let mut data = TcpStream::connect_timeout(&data_sock, Duration::from_secs(30))
        .with_context(|| format!("data dial {data_addr}"))?;

    write_ftp(&mut conn, &format!("STOR {remote_name}"))?;
    let code = read_ftp(&mut br)?;
    if !(code.starts_with("150") || code.starts_with("125")) {
        bail!("ftp STOR: {}", code.trim());
    }
    std::io::copy(&mut reader, &mut data)?;
    drop(data);
    let done = read_ftp(&mut br)?;
    if !(done.starts_with("226") || done.starts_with("250")) {
        bail!("ftp transfer: {}", done.trim());
    }
    let _ = write_ftp(&mut conn, "QUIT");
    Ok(())
}

fn write_ftp(conn: &mut TcpStream, line: &str) -> Result<()> {
    conn.write_all(format!("{line}\r\n").as_bytes())?;
    Ok(())
}

fn read_ftp(br: &mut BufReader<TcpStream>) -> Result<String> {
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        br.read_line(&mut line)?;
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        let done = line.len() >= 4 && line.as_bytes()[3] == b' ';
        let short = line.len() < 4;
        lines.push(line);
        if done || short {
            break;
        }
    }
    Ok(lines.join("\n"))
}

fn ftp_cmd(
    conn: &mut TcpStream,
    br: &mut BufReader<TcpStream>,
    cmd: &str,
    want: &str,
) -> Result<()> {
    write_ftp(conn, cmd)?;
    let resp = read_ftp(br)?;
    if !resp.starts_with(want) {
        bail!("ftp {cmd}: {}", resp.trim());
    }
    Ok(())
}

fn parse_pasv(line: &str) -> Result<(String, u16)> {
    let start = line
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("bad PASV: {line}"))?;
    let end = line[start..]
        .find(')')
        .ok_or_else(|| anyhow::anyhow!("bad PASV: {line}"))?
        + start;
    let parts: Vec<&str> = line[start + 1..end].split(',').collect();
    if parts.len() < 6 {
        bail!("bad PASV: {line}");
    }
    let mut nums = [0u16; 6];
    for i in 0..6 {
        nums[i] = parts[i]
            .trim()
            .parse()
            .with_context(|| format!("bad PASV: {line}"))?;
    }
    let host = format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3]);
    let port = nums[4] * 256 + nums[5];
    Ok((host, port))
}
