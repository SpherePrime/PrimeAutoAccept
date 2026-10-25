use std::path::PathBuf;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

pub fn find_pid(exe_name: &str) -> Option<u32> {
    find_pids(&[exe_name])[0]
}

pub fn find_pids(exe_names: &[&str]) -> Vec<Option<u32>> {
    let mut found = vec![None; exe_names.len()];
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return found;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while has_entry && found.iter().any(Option::is_none) {
        let name_len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
        for (slot, wanted) in found.iter_mut().zip(exe_names) {
            if slot.is_none() && name.eq_ignore_ascii_case(wanted) {
                *slot = Some(entry.th32ProcessID);
            }
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    close(snapshot);
    found
}

pub fn executable_path(pid: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = vec![0u16; 1024];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    close(handle);
    result.ok()?;
    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..length as usize],
    )))
}

fn close(handle: HANDLE) {
    let _ = unsafe { CloseHandle(handle) };
}
