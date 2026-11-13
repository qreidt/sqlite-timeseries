# Data lifecycle

[← Contents](README.md)

Retention and compression act on whole chunks, which is what makes them cheap. Both must be atomic and must work around SQLite's rule that a table cannot be dropped while another statement on the connection is running.

## Retention

| ID | Requirement | Priority |
| --- | --- | --- |
| LC-01 | `drop_chunks(table, older_than [, newer_than])` removes chunks whose whole range is older than the threshold, never part of a chunk. Thresholds accept an interval (relative to now) or an absolute time. It returns the dropped chunk names. | Must |
| LC-02 | Drops are two-phase: the catalog marks the chunk `dropped`, which hides it from queries at once, and the table is physically dropped when no other statement is active (same call when possible, otherwise the next `run_jobs()` or an explicit cleanup call). | Must |
| LC-03 | Dropping raw chunks never invalidates or deletes continuous aggregate data. | Must |
| LC-04 | The docs state that dropped pages go to the freelist and the file does not shrink without `VACUUM` or `auto_vacuum=INCREMENTAL`. The retention policy can optionally run `incremental_vacuum`. | Should |

## Compression

| ID | Requirement | Priority |
| --- | --- | --- |
| LC-05 | `set_compression(table, segment_by, order_by [, batch_size])` stores per-hypertable settings. `order_by` defaults to the time column; `batch_size` defaults to 1,000 rows, as in TimescaleDB ([TimescaleDB docs](https://www.tigerdata.com/docs/use-timescale/latest/hypercore/secondary-indexes)). | Must |
| LC-06 | `compress_chunk(chunk)` rewrites a chunk into a block file with one block per batch (see [Block files](08-block-files.md)). Segment-by values, row count, min and max time and column offsets go to the catalog. The original rows are removed two-phase, as in LC-02. | Must |
| LC-07 | `decompress_chunk(chunk)` restores an ordinary chunk with identical rows. | Must |
| LC-08 | Encodings, modelled on TimescaleDB's ([compression methods](https://www.tigerdata.com/docs/learn/columnar-storage/compression-methods)): delta-of-delta with zigzag and simple-8b-style bit packing plus run-length encoding for timestamps and integers; Gorilla XOR for floats; dictionary for low-cardinality text; run-length bitmaps for booleans and NULLs; a general-purpose codec (zstd or LZ4) as fallback. | Must |
| LC-09 | Round-trips are lossless for every SQLite storage class, including columns that mix storage classes row by row, NaN, ±infinity, −0.0 and the int64 extremes. Mixed columns fall back to a typed generic encoding. | Must |
| LC-10 | Every block file and encoded column starts with a header: magic, format version, encoding id, value count, NULL-bitmap flag. Byte order is fixed (little-endian), so files move between machines. | Must |
| LC-11 | Scans of compressed chunks decode batch by batch, skip batches using min/max time and segment-by equality, and decode only the columns the query uses. | Must |
| LC-12 | Queries read compressed and uncompressed chunks through the same virtual table with no SQL changes. | Must |
| LC-13 | Inserts into a compressed chunk land in an uncompressed side table for that chunk and are merged on recompression. | Should |
| LC-14 | UPDATE and DELETE on compressed rows either decompress the affected batches or fail with a clear error. The minimum for v1 is the clear error. | Must |
| LC-15 | Decoders treat blobs as untrusted input: bounded allocations, no panics, and `SQLITE_CORRUPT` on malformed data. | Must |
