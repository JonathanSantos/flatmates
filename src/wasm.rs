//! Borda WebAssembly, sem dependências: o navegador escreve o JSON na
//! memória, chama `flatmates_render` e lê a resposta.
use crate::{render, render_steady, Options, Output};

/// Reserva `len` bytes para o chamador escrever o JSON.
#[no_mangle]
pub extern "C" fn flatmates_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Libera um bloco devolvido por `flatmates_alloc` ou `flatmates_render`.
///
/// # Safety
/// `ptr` e `cap` precisam ter vindo de uma dessas duas funções.
#[no_mangle]
pub unsafe extern "C" fn flatmates_free(ptr: *mut u8, cap: usize) {
    drop(Vec::from_raw_parts(ptr, 0, cap.max(1)));
}

fn reply(result: Result<Output, String>) -> *mut u8 {
    let text = match result {
        Ok(out) => serde_json::json!({ "svg": out.svg, "warnings": out.warnings }),
        Err(e) => serde_json::json!({ "error": e }),
    }
    .to_string();
    let mut buf = Vec::with_capacity(text.len() + 4);
    buf.extend_from_slice(&(text.len() as u32).to_le_bytes());
    buf.extend_from_slice(text.as_bytes());
    buf.shrink_to_fit();
    let out = buf.as_mut_ptr();
    std::mem::forget(buf);
    out
}

/// Renderiza o JSON em `ptr..ptr+len`. Devolve um bloco que começa com 4
/// bytes (tamanho, little-endian) seguidos de um JSON:
/// `{"svg": "...", "warnings": [...]}` ou `{"error": "..."}`.
///
/// # Safety
/// `ptr` precisa apontar para `len` bytes válidos.
#[no_mangle]
pub unsafe extern "C" fn flatmates_render(ptr: *const u8, len: usize) -> *mut u8 {
    let input = std::slice::from_raw_parts(ptr, len);
    reply(std::str::from_utf8(input).map_err(|e| e.to_string()).and_then(|j| render(j, &Options::default())))
}

/// Como `flatmates_render`, com opções: `time` é o instante do ciclo de
/// movimento (0 a 1); em `flags`, o bit 0 desenha o esqueleto e o bit 1
/// mantém o enquadramento estável entre quadros.
///
/// # Safety
/// `ptr` precisa apontar para `len` bytes válidos.
#[no_mangle]
pub unsafe extern "C" fn flatmates_frame(ptr: *const u8, len: usize, time: f64, flags: u32) -> *mut u8 {
    let input = std::slice::from_raw_parts(ptr, len);
    let opts = Options { rig: flags & 1 != 0, time, ..Options::default() };
    reply(std::str::from_utf8(input).map_err(|e| e.to_string()).and_then(|j| {
        if flags & 2 != 0 {
            render_steady(j, &opts, 8)
        } else {
            render(j, &opts)
        }
    }))
}
