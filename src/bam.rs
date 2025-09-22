use bam::bam_reader::{IndexedReaderBuilder, IndexedReader, RegionViewer};
use bam::index::{Chunk, VirtualOffset};
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use std::fs::File;

pub fn read_bam_records_interval<'a>(
    bam_index_reader: &'a mut IndexedReader<File>,
    start_virtual_offset: u64,
    end_virtual_offset: u64,
) -> Result<RegionViewer<'a, File>, std::io::Error> {
    let start_voffset = VirtualOffset::from_raw(start_virtual_offset);
    let end_voffset = VirtualOffset::from_raw(end_virtual_offset);
    let intervals = [Chunk::new(start_voffset, end_voffset)];

    // Return an iterator over the records in the specified interval
    let records = bam_index_reader.fetch_chunks(intervals);
    Ok(records)
}

/// Convert a list of linear index offsets into `Chunk`s.
/// Assumes offsets are monotonic increasing and non-zero.
/// The last offset is extended to EOF (`VirtualOffset::MAX`).
pub fn linear_offsets_to_chunks(offsets: &[u64]) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    // Build chunks between consecutive offsets
    for i in 0..offsets.len().saturating_sub(1) {
        chunks.push(Chunk::new(
            VirtualOffset::from_raw(offsets[i]),
            VirtualOffset::from_raw(offsets[i + 1]),
        ));
    }

    // Add the last interval to EOF
    if let Some(&last) = offsets.last() {
        chunks.push(Chunk::new(
            VirtualOffset::from_raw(last),
            VirtualOffset::MAX,
        ));
    }

    chunks
}

/// Read all intervals in parallel and return the total number of records
pub fn read_bam_parallel_with_threads(
    bam_path: &str,
    chunks: &[Chunk],
    num_threads: usize,
) -> Result<usize, std::io::Error> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap(); // panic if failed

    // Run the parallel computation in this thread pool
    pool.install(|| {
        let total: Result<usize, std::io::Error> = chunks
            .par_iter()
            .map(|chunk| {
                let chunk = chunk.clone();
                let mut index_reader_builder = IndexedReaderBuilder::new();
                // index_reader_builder.additional_threads((&num_threads - 1) as u16); // each thread uses 4 additional thread for decompression
                index_reader_builder.additional_threads((3) as u16); // each thread uses 4 additional thread for decompression
                let mut reader: IndexedReader<File> = index_reader_builder.from_path(bam_path)?;
                let mut records = reader.fetch_chunks([chunk]);

                let mut count = 0;
                for r in &mut records {
                    r?;
                    count += 1;
                }
                Ok(count)
            })
            .sum();

        total
    })
}

#[cfg(test)]
mod tests {
    use crate::index::{get_linear_index, parse_index};

    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn test_read_bam_records_interval() -> Result<(), Box<dyn std::error::Error>> {
        // Path to test BAM
        let test_bam_path = "/Users/yifeiwan/Projects/bamstorm/test.bam";

        // Open IndexedReader
        let mut reader = IndexedReader::from_path(test_bam_path)?;

        // Virtual offsets
        let start_virtual_offset = 229244928; // example start
        let end_virtual_offset = 22429409449; // example end

        // Call the function
        let mut records =
            read_bam_records_interval(&mut reader, start_virtual_offset, end_virtual_offset)?;

        // Iterate and count a few records
        let mut count = 0;
        for rec in &mut records {
            let rec = rec?;
            // Optionally check something about the record
            println!("Record: {:?}", rec);
            count += 1;

            // Just limit output for test
            if count >= 5 {
                break;
            }
        }

        assert!(count > 0, "Should read at least one record from BAM");

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_pair_successive_nonzero_voffsets() {
        let offsets: Vec<u64> = vec![0, 100, 200, 0, 400];

        // Expect pairs: (100,200), (200,400)
        let chunks = linear_offsets_to_chunks(&offsets);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].start().raw(), 100);
        assert_eq!(chunks[0].end().raw(), 200);
        assert_eq!(chunks[1].start().raw(), 200);
        assert_eq!(chunks[1].end().raw(), 400);
    }

    #[test]
    #[ignore]
    fn test_single_nonzero_linear_voffsets() {
        let offsets: Vec<u64> = vec![0, 0, 123];
        let chunks = linear_offsets_to_chunks(&offsets);
        assert!(chunks.is_empty());
    }

    #[test]
    #[ignore]
    fn test_no_nonzero_linear_voffsets() {
        let offsets: Vec<u64> = vec![0, 0, 0];
        let chunks = linear_offsets_to_chunks(&offsets);
        assert!(chunks.is_empty());
    }

    #[test]
    #[ignore]
    fn test_read_bam_parallel_with_offsets() {
        // Test BAM path
        let test_bam_path = "/Users/yifeiwan/Projects/bamstorm/test.bam";

        // Linear index offsets (from your message)
        let offsets = vec![229244928, 22429409449, 104080987383, 104793051268];

        // Convert offsets into chunks
        let chunks = linear_offsets_to_chunks(&offsets);

        // Optional: check the chunks
        for (i, c) in chunks.iter().enumerate() {
            println!(
                "Chunk {}: start={}, end={}",
                i,
                c.start().raw(),
                c.end().raw()
            );
        }

        // Read BAM in parallel using chunks
        let total_reads =
            read_bam_parallel_with_threads(test_bam_path, &chunks, 4).expect("Failed to read BAM in parallel");

        println!("Total reads in all chunks: {}", total_reads);

        // Assert we got some reads
        assert_eq!(total_reads, 38235, "Expected 38235 reads");
    }

    #[test]
    fn test_read_bam_parallel_read_all() {
        let start = Instant::now(); // start timer
        // Test BAM path
        // let test_bam_path = "/Users/yifeiwan/Projects/BamStorm/wgEncodeUwRepliSeqBg02esG1bAlnRep1.bam";
        // let test_bai_path = "/Users/yifeiwan/Projects/BamStorm/wgEncodeUwRepliSeqBg02esG1bAlnRep1.bam.bai";
        // let test_bam_path = "/Users/yifeiwan/Projects/bamstorm/test.bam";
        // let test_bai_path = "/Users/yifeiwan/Projects/bamstorm/test.bam.bai";
        let test_bam_path = "/Users/yifeiwan/Projects/bamstorm/chr1.bam";
        let test_bai_path = "/Users/yifeiwan/Projects/bamstorm/chr1.bam.bai";

        // Linear index offsets (from your message)
        let index = parse_index(&test_bai_path).unwrap();
        let offsets = get_linear_index(&index).unwrap();
        println!("Number of offsets: {}", offsets.len());
        // println!("Offset: {:?}", &offsets);
        // assert_eq!(offsets.len(), 191457, "Expected 191457 offsets");

        // Convert offsets into chunks
        let chunks = linear_offsets_to_chunks(&offsets);
        // println!("Head of chunks:");
        // for c in chunks.iter().take(5) {
        //     println!("({}, {}) ", c.start().raw(), c.end().raw());
        // }
        let chunks = chunks[0..1000].to_vec(); // limit to first X chunks for testing

        // Read BAM in parallel using chunks
        let num_threads: usize = 1;
        println!("Using {} threads", num_threads);
        let total_reads =
            read_bam_parallel_with_threads(test_bam_path, &chunks, num_threads).expect("Failed to read BAM in parallel");

        println!("Total reads in all chunks: {}", total_reads);
        let duration = start.elapsed(); // end timer
        println!("Time elapsed in expensive_function() is: {:?}", duration);

        // Assert we got some reads
        assert!(total_reads > 0, "Expected at least one read");
    }
}
