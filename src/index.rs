#![allow(unused_imports)]
use std::fmt;
use std::fs::File;
use std::io::{self, Read};

const IDX_MAGIC: &[u8; 4] = b"BAI\x01";

#[derive(Debug)]
struct InnerChunk {
    start: u64,
    end: u64,
}

#[derive(Debug)]
pub struct Bin {
    bin_id: u32,
    chunks: Vec<InnerChunk>,
}

#[derive(Debug)]
pub struct Reference {
    pub bins: Vec<Bin>,
    pub linear_index: Vec<u64>,
}

#[derive(Debug)]
pub struct Index {
    pub magic: [u8; 4],
    pub n_ref: u32,
    pub references: Vec<Reference>,
    pub n_no_coor: u64,
}

// Inline function to get coffect from the virtual file offset
#[inline]
fn get_coffset(virtual_offset: u64) -> u64 {
    virtual_offset >> 16
}

#[inline]
fn get_uoffset(virtual_offset: u64) -> u64 {
    virtual_offset & 0xFFFF
}

#[inline]
fn make_virtual_offset(coffset: u64, uoffset: u64) -> u64 {
    (coffset << 16) | (uoffset)
}

pub fn parse_index(bam_index_path: &str) -> Result<Index, std::io::Error> {
    let mut magic_buffer: [u8; 4] = [0u8; 4];
    let mut n_ref_buffer: [u8; 4] = [0u8; 4]; // Buffer for number of references
    let mut file = File::open(bam_index_path)?; // Open the file

    // Read the first 4 bytes into the buffer
    file.read_exact(&mut magic_buffer).unwrap_or_else(|err| {
        panic!("Error reading magic number from index file: {}", err);
    });

    assert_eq!(&magic_buffer, IDX_MAGIC);

    // Check if the magic number matches
    if &magic_buffer != IDX_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid index file magic number",
        ));
    }

    // Read the next 4 bytes for the number of references
    file.read_exact(&mut n_ref_buffer).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Error reading number of references: {}", err),
        )
    })?;

    let n_ref = u32::from_le_bytes(n_ref_buffer);
    // println!("Number of references: {}", n_ref);

    // Initialize a vector to hold references
    let mut ref_sets = Vec::with_capacity(n_ref as usize);

    for reference in 0..n_ref {
        // Read number of bins
        let mut n_bin_buffer: [u8; 4] = [0u8; 4];
        file.read_exact(&mut n_bin_buffer).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Error reading number of bins for reference {}: {}",
                    reference, err
                ),
            )
        })?;

        let n_bin = u32::from_le_bytes(n_bin_buffer);
        // println!("Reference {}: Number of bins: {}", reference, n_bin);

        let mut bins = Vec::with_capacity(n_bin as usize);

        for _ in 0..n_bin {
            // Read bin ID
            let mut bin_id_buffer: [u8; 4] = [0u8; 4];
            file.read_exact(&mut bin_id_buffer).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Error reading bin ID for reference {}: {}", reference, err),
                )
            })?;
            let bin_id = u32::from_le_bytes(bin_id_buffer);

            // Read number of chunks
            let mut n_chunk_buffer: [u8; 4] = [0u8; 4];
            file.read_exact(&mut n_chunk_buffer).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Error reading number of chunks for reference {} - bin {}: {}",
                        reference, bin_id, err
                    ),
                )
            })?;
            let n_chunk = u32::from_le_bytes(n_chunk_buffer);

            // Initialize a vector to hold chunks
            let mut chunks = Vec::with_capacity(n_chunk as usize);

            for _ in 0..n_chunk {
                // Read chunk start and end
                let mut chunk_start_buffer: [u8; 8] = [0u8; 8];
                let mut chunk_end_buffer: [u8; 8] = [0u8; 8];
                file.read_exact(&mut chunk_start_buffer).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Error reading chunk start for reference {} - bin {}: {}",
                            reference, bin_id, err
                        ),
                    )
                })?;
                file.read_exact(&mut chunk_end_buffer).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Error reading chunk end for reference {} - bin {}: {}",
                            reference, bin_id, err
                        ),
                    )
                })?;

                let chunk_start = u64::from_le_bytes(chunk_start_buffer);
                let chunk_end = u64::from_le_bytes(chunk_end_buffer);

                chunks.push(InnerChunk {
                    start: chunk_start,
                    end: chunk_end,
                });
            }

            bins.push(Bin { bin_id, chunks });
        }

        // Read linear index
        let mut n_intv_buffer: [u8; 4] = [0u8; 4];
        file.read_exact(&mut n_intv_buffer).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Error reading number of linear index intervals for reference {}: {}",
                    reference, err
                ),
            )
        })?;
        let n_intv = u32::from_le_bytes(n_intv_buffer);
        // println!(
            // "Reference {}: Number of linear index intervals: {}",
            // reference, n_intv
        // );

        let mut linear_index = Vec::with_capacity(n_intv as usize);
        for intv_id in 0..n_intv {
            let mut offset_buffer: [u8; 8] = [0u8; 8];
            file.read_exact(&mut offset_buffer).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Error reading linear index offset for reference {} - n_intv {}: {}",
                        reference, intv_id, err
                    ),
                )
            })?;
            let offset = u64::from_le_bytes(offset_buffer);
            linear_index.push(offset);
        }

        ref_sets.push(Reference { bins, linear_index });
    }
    // Read number of unplaced reads
    let mut n_no_coor_buffer: [u8; 8] = [0u8; 8];
    file.read_exact(&mut n_no_coor_buffer)?;
    let n_no_coor = u64::from_le_bytes(n_no_coor_buffer);
    println!("Number of unplaced reads: {}", n_no_coor);

    // Construct and return the Index struct
    Ok(Index {
        magic: magic_buffer,
        n_ref,
        references: ref_sets,
        n_no_coor,
    })
}

#[derive(Debug)]
pub enum IndexError {
    EmptyIndex,
    NoLinearIndexFound,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexError::EmptyIndex => write!(f, "Index has no references"),
            IndexError::NoLinearIndexFound => write!(f, "No valid index found in linear index"),
        }
    }
}

impl std::error::Error for IndexError {}

/// Convert a BAM Index into a list of virtual offset
pub fn get_linear_index(index: &Index) -> Result<Vec<u64>, IndexError> {
    let mut indices = Vec::new();

    if index.references.is_empty() {
        return Err(IndexError::EmptyIndex);
    }

    for reference in &index.references {
        let lin_idx = &reference.linear_index;
        if lin_idx.is_empty() {
            continue;
        }

        let mut _prev_id: u64 = 0;
        // Get all linear indices of this reference
        for idx in lin_idx {
            // if *idx == _prev_id {
            //     println!("duplicate {:?}", _prev_id); // skip duplicates
            // }
            if *idx != 0 && *idx > _prev_id {
                indices.push(*idx);
                _prev_id = *idx;
            }
        }
    }

    if indices.is_empty() {
        return Err(IndexError::NoLinearIndexFound);
    }

    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_parse_index() {
        // Path to a small test BAM index file (.bai)
        let test_index_path = "/Users/yifeiwan/Projects/bamstorm/test.bam.bai";

        // Call the parse_index function
        let result = parse_index(test_index_path);

        // The function should return Ok(Index)
        assert!(
            result.is_ok(),
            "parse_index should succeed for a valid BAI file"
        );

        let index = result.unwrap();

        // Check the magic number matches expected
        assert_eq!(
            &index.magic, IDX_MAGIC,
            "Magic number should match IDX_MAGIC"
        );

        // Check the number of references is > 0
        assert!(
            index.n_ref > 0,
            "Number of references should be greater than 0"
        );

        // Check that each reference has a linear index
        for reference in &index.references {
            assert!(
                !reference.linear_index.is_empty(),
                "Linear index should not be empty"
            );
        }

        // Optionally, print info for visual inspection
        println!(
            "Parsed index: {} references, {} unplaced reads",
            index.n_ref, index.n_no_coor
        );
    }

    #[test]
    #[ignore]
    fn test_virtual_offset_roundtrip() {
        // Make up some test values
        let coffset: u64 = 0x1234_5678_9ABC;
        let uoffset: u64 = 0xDEF0; // u64 to match make_virtual_offset signature

        // Assemble the virtual offset
        let voffset = make_virtual_offset(coffset, uoffset);

        // Check if funcs can extract them back correctly
        assert_eq!(get_coffset(voffset), coffset);
        assert_eq!(get_uoffset(voffset), uoffset);

        // Reassemble and check if func gets the same virtual offset
        let voffset2 = make_virtual_offset(get_coffset(voffset), get_uoffset(voffset));
        assert_eq!(voffset, voffset2);
    }

    #[test]
    #[ignore]
    fn test_zero_offsets() {
        let voffset = make_virtual_offset(0, 0);
        assert_eq!(voffset, 0);
        assert_eq!(get_coffset(voffset), 0);
        assert_eq!(get_uoffset(voffset), 0);
    }

    #[test]
    #[ignore]
    fn test_max_uoffset() {
        let coffset: u64 = 0x1111_2222_3333;
        let uoffset: u64 = 0xDEF0; // u64 to match make_virtual_offset signature
        let voffset = make_virtual_offset(coffset, uoffset);

        assert_eq!(get_coffset(voffset), coffset);
        assert_eq!(get_uoffset(voffset), uoffset);
    }

    #[test]
    #[ignore]
    // Test with real BAI file
    fn test_real_bai_file() {
        let bai_path = "./test.bam.bai";
        let index = parse_index(bai_path).expect("Failed to read index");
        // Print the 1st and 2nd linear index of the first reference for debugging
        println!(
            "First reference, first two linear index entries: {:?}",
            &index.references[0].linear_index[0..4]
        );
    }

    #[test]
    #[ignore]
    fn test_get_linear_index() {
        // Prepare a fake Index for testing
        let index = Index {
            magic: *b"BAI\x01",
            n_ref: 2,
            n_no_coor: 0,
            references: vec![
                Reference {
                    bins: vec![],
                    linear_index: vec![0, 100, 200, 0, 400], // mixed zeros and non-zero
                },
                Reference {
                    bins: vec![],
                    linear_index: vec![0, 0, 500, 600],
                },
            ],
        };

        // Call the function
        let linear_offsets = get_linear_index(&index).expect("Should return linear index offsets");

        // Check that all non-zero offsets are collected in order
        assert_eq!(linear_offsets, vec![100, 200, 400, 500, 600]);

        // Print for visual verification (optional)
        println!("Linear offsets: {:?}", linear_offsets);
    }

    #[test]
    #[ignore]
    fn test_empty_index_error() {
        // Empty references should return an error
        let index = Index {
            magic: *b"BAI\x01",
            n_ref: 0,
            n_no_coor: 0,
            references: vec![],
        };

        let result = get_linear_index(&index);
        assert!(matches!(result, Err(IndexError::EmptyIndex)));
    }

    #[test]
    #[ignore]
    fn test_no_linear_index_found_error() {
        // All linear_index entries are zero
        let index = Index {
            magic: *b"BAI\x01",
            n_ref: 1,
            n_no_coor: 0,
            references: vec![Reference {
                bins: vec![],
                linear_index: vec![0, 0, 0],
            }],
        };

        let result = get_linear_index(&index);
        assert!(matches!(result, Err(IndexError::NoLinearIndexFound)));
    }

    #[test]
    fn test_get_offsets_from_real_bai() {
        let test_bai_path = "/Users/yifeiwan/Projects/bamstorm/test.bam.bai";
        let index = parse_index(test_bai_path).unwrap();
        let offsets = get_linear_index(&index).unwrap();
        // println!("offsets: {:?}", offsets);
        assert!(!offsets.is_empty(), "Offsets should not be empty");
        assert_eq!(offsets.len(), 176782, "Expected 176782 offsets");
        println!("Number of offsets: {}", offsets.len());
        
    }
}
