/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
//! # hexify
//!
//! `hexify` is a Rust library for formatting octet slices (`[u8]`) into hexadecimal (`hex`) strings.
//! It provides utilities to convert octets into hex strings.
//!
//! ## Installation
//!
//! Add `hexify` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! hexify = "0.0.1"
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use hexify::{format_hex};
//!
//! let data = [0x42, 0xA4, 0xAE, 0x09, 0xAF, 0x00, 0x01, 0x00, 0x00, 0x04, 0x03, 0x00, 0x00];
//!
//! let output = format_hex(&data);
//! println!("{}", output); // Outputs: 42 A4 AE 09 AF 00 01 00 00 04 03 00 00
//! ```


/// Formats a octet slice into an uppercase, space-separated hexadecimal string.
///
/// This is a convenience function that formats the input octets into an uppercase
/// hexadecimal string with spaces separating each octet.
///
/// # Parameters
///
/// - `buf`: A slice of octets (`[u8]`) to format.
///
/// # Returns
///
/// A `String` containing the uppercase, space-separated hexadecimal representation.
///
/// # Examples
///
/// ```rust
/// use hexify::format_hex;
///
/// let data = [0x42, 0xA4, 0xAE];
/// let hex = format_hex(&data);
/// assert_eq!(hex, "00000000  42 A4 AE                                          B..");
/// ```
pub fn format_hex(buf: &[u8]) -> String {
    format_hex_with_prefix_and_separator(buf, "", " ", 16, 8)
}

pub fn format_hex_with_prefix_and_separator(
    buf: &[u8],
    prefix: &str,
    separator: &str,
    octets_per_line: usize,
    group_size: usize,
) -> String {
    // Ensure group_size is not zero and does not exceed bytes_per_line
    assert!(
        group_size > 0 && group_size <= octets_per_line,
        "group_size must be greater than 0 and less than or equal to octets_per_line"
    );

    let mut result = String::new();

    for (i, chunk) in buf.chunks(octets_per_line).enumerate() {
        // Calculate the offset
        let offset = i * octets_per_line;
        // Write the offset as 8-digit hexadecimal, padded with zeros
        result.push_str(&format!("{:08X}  ", offset));

        // Initialize a buffer to collect ASCII characters
        let mut ascii_repr = String::new();

        // Iterate over bytes in the chunk
        for (j, byte) in chunk.iter().enumerate() {
            // Insert extra space after every `group_size` bytes, except after the last byte
            if j > 0 && j % group_size == 0 {
                result.push(' '); // Extra space between groups
                result.push(' '); // Extra space between groups
            } else if j > 0 {
                result.push_str(separator); // Regular separator
            }

            // Append the formatted byte
            result.push_str(&format!("{prefix}{:02X}", byte));

            // Collect ASCII representation
            if byte.is_ascii_graphic() || byte == &b' ' {
                ascii_repr.push(*byte as char);
            } else {
                ascii_repr.push('.'); // Non-printable characters represented by '.'
            }
        }

        // Calculate padding for incomplete groups to align ASCII representation
        if chunk.len() < octets_per_line {
            let missing_bytes = octets_per_line - chunk.len();
            let extra_spaces = if group_size > 0 {
                // Calculate number of full groups missing
                let full_groups_missing = missing_bytes / group_size;
                // Number of remaining bytes after full groups
                let remaining_bytes = missing_bytes % group_size;

                // Each missing byte adds: prefix.len() + 2 (for {:02X}) + separator.len()
                let per_byte_space = prefix.len() + 2 + separator.len();

                // Each missing group adds an extra space
                let per_group_space = 1; // One space between groups

                // Total padding
                full_groups_missing * (group_size * per_byte_space + per_group_space)
                    + remaining_bytes * per_byte_space
            } else {
                // If group_size is zero, no extra grouping
                missing_bytes * (prefix.len() + 2 + separator.len())
            };

            result.push_str(&" ".repeat(extra_spaces));
        }

        // Add two spaces before ASCII representation
        result.push_str("  ");

        // Append ASCII characters
        result.push_str(&ascii_repr);

        // Add a newline if it's not the last line
        if i < (buf.len() + octets_per_line - 1) / octets_per_line - 1 {
            result.push('\n');
        }
    }

    result
}

pub fn format_hex_u32_be(num: u32) -> String {
    format_hex_with_prefix_and_separator(&num.to_be_bytes(), "0x", ",", 16, 8)
}

pub fn format_hex_dump_comparison(buf1: &[u8], buf2: &[u8]) -> String {
    format_hex_dump_comparison_interleaved(buf1, buf2, "", " ", 16, 8)
}

/// Computes `group_size` as a power of two within the range [4, 16].
///
/// The function calculates `group_size` based on `octets_per_line / 2`,
/// clamps it between 4 and 16, and then adjusts it to the nearest lower
/// power of two within this range.
///
/// # Parameters
///
/// - `octets_per_line`: The number of octets (bytes) per line.
///
/// # Returns
///
/// - `usize`: The computed `group_size` (4, 8, or 16).
fn compute_group_size(octets_per_line: usize) -> usize {
    1 << ((octets_per_line / 2).clamp(4, 16).ilog2())
}

pub fn format_hex_dump_comparison_width(
    encountered_buf: &[u8],
    expected_buf: &[u8],
    octets_per_line: usize,
) -> String {
    let group_size = compute_group_size(octets_per_line);

    format_hex_dump_comparison_interleaved(
        encountered_buf,
        expected_buf,
        "",
        " ",
        octets_per_line,
        group_size,
    )
}

/// Formats two byte buffers into an interleaved hex dump comparison with a shared offset and ASCII representations.
/// Differing hex bytes in `buf2` are highlighted using ANSI colors.
///
/// # Parameters
///
/// - `encountered_buf`: The second octet buffer to format (the one buffer to compare to the expected).
/// - `expected_buf`: The first octet buffer to format (usually the correct or expected buffer).
/// - `prefix`: A string slice to prefix each hex byte (e.g., "0x").
/// - `separator`: A string slice to separate hex bytes (e.g., " ").
/// - `bytes_per_line`: Number of bytes to display per line before inserting a line break.
/// - `group_size`: Number of bytes after which to insert an extra space within a line.
///
/// # Returns
///
/// - `String`: The formatted interleaved hex dump comparison with ANSI coloring.
fn format_hex_dump_comparison_interleaved(
    encountered_buf: &[u8],
    expected_buf: &[u8],
    prefix: &str,
    separator: &str,
    bytes_per_line: usize,
    group_size: usize,
) -> String {
    // Ensure group_size is valid
    assert!(
        group_size > 0 && group_size <= bytes_per_line,
        "group_size must be greater than 0 and less than or equal to bytes_per_line"
    );

    let mut result = String::new();

    // Determine the number of lines based on the longer buffer
    let total_lines =
        (expected_buf.len().max(encountered_buf.len()) + bytes_per_line - 1) / bytes_per_line;

    for line in 0..total_lines {
        let offset = line * bytes_per_line;
        let offset_str = format!("{:08X}", offset);

        let expected_chunk = &expected_buf
            [line * bytes_per_line..expected_buf.len().min((line + 1) * bytes_per_line)];
        let encountered_chunk = &encountered_buf
            [line * bytes_per_line..encountered_buf.len().min((line + 1) * bytes_per_line)];

        let formatted_hex1 = format_hex_octets(expected_chunk, prefix, separator, group_size);
        let ascii1 = format_ascii(expected_chunk);

        let formatted_hex2 = format_hex_bytes_with_diff(
            encountered_chunk,
            expected_chunk,
            prefix,
            separator,
            group_size,
        );
        let ascii2 = format_ascii(encountered_chunk);

        if !expected_chunk.is_empty() {
            result.push_str(&format!(
                "{:<10}  {:<}  {}!\n",
                offset_str, formatted_hex1, ascii1
            ));
        }

        if !encountered_chunk.is_empty() {
            result.push_str(&format!(
                "{:<10}  {:<}  {}!\n",
                offset_str, formatted_hex2, ascii2
            ));
        }
    }

    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Formats a slice of octets into a grouped hex string without coloring.
///
/// # Parameters
///
/// - `buf`: The octet slice to format.
/// - `prefix`: Prefix for each hex octet.
/// - `separator`: Separator between hex octets.
/// - `group_size`: Number of octets after which to insert extra space.
///
/// # Returns
///
/// - `String`: The formatted hex string.
fn format_hex_octets(buf: &[u8], prefix: &str, separator: &str, group_size: usize) -> String {
    let mut hex_str = String::new();

    for (j, octet) in buf.iter().enumerate() {
        // Insert two spaces after every `group_size` bytes, except before the first byte
        if j > 0 && j % group_size == 0 {
            hex_str.push(' '); // First space between groups
            hex_str.push(' '); // Second space between groups
        } else if j > 0 {
            hex_str.push_str(separator); // Regular separator
        }

        // Append the formatted byte
        hex_str.push_str(&format!("{prefix}{:02X}", octet));
    }

    hex_str
}

/// Formats a slice of octets into a grouped hex string with coloring for differing bytes in buf2.
///
/// Differing bytes between `encountered_buf` and `expected_buf` are highlighted in red.
///
/// # Parameters
///
/// - `encountered_buf`: The octet slice to format with possible coloring.
/// - `expected_buf`: The expected octet slice for comparison.
/// - `prefix`: Prefix for each hex octet.
/// - `separator`: Separator between hex octets.
/// - `group_size`: Number of octets after which to insert extra space.
///
/// # Returns
///
/// - `String`: The formatted hex string with ANSI coloring for differences.
fn format_hex_bytes_with_diff(
    encountered_buf: &[u8],
    expected_buf: &[u8],
    prefix: &str,
    separator: &str,
    group_size: usize,
) -> String {
    let mut hex_str = String::new();

    for (j, octet) in encountered_buf.iter().enumerate() {
        // Insert two spaces after every `group_size` octets, except before the first octet
        if j > 0 && j % group_size == 0 {
            hex_str.push(' '); // First space between groups
            hex_str.push(' '); // Second space between groups
        } else if j > 0 {
            hex_str.push_str(separator); // Regular separator
        }

        // Determine if the current byte differs from buf1
        let differing = j < expected_buf.len() && octet != &expected_buf[j];

        if differing {
            hex_str.push_str("\x1b[31m"); // Start red color
        }

        hex_str.push_str(&format!("{prefix}{:02X}", octet));

        if differing {
            hex_str.push_str("\x1b[0m"); // Reset color
        }
    }

    hex_str
}

/// Formats a slice of octets into an ASCII string with visible characters.
///
/// Non-printable characters are represented by '.'.
///
/// # Parameters
///
/// - `buf`: The octet slice to format.
///
/// # Returns
///
/// - `String`: The ASCII representation.
fn format_ascii(buf: &[u8]) -> String {
    buf.iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// Asserts that two octet slices are equal. If not, panics and displays a formatted hex dump comparison.
///
/// # Parameters
///
/// - `buf_to_test`: The first octet slice to compare.
/// - `expected_buf`: The second octet slice to compare.
///
/// # Panics
///
/// Panics with a detailed hex dump comparison if `buf_to_test` and `expected_buf` are not equal.
pub fn assert_eq_slices(buf_to_test: &[u8], expected_buf: &[u8]) {
    if buf_to_test == expected_buf {
        #[cfg(feature = "log_equal")]
        {
            use log::trace;
            trace!("assert: slices are equal: {}", format_hex(buf_to_test));
        }
        return;
    }

    let formatted_dump = format_hex_dump_comparison_width(buf_to_test, expected_buf, 16);

    // Panic with the formatted comparison
    panic!(
        "octet slices are not equal!\n\nComparison:\n{}",
        formatted_dump
    );
}
