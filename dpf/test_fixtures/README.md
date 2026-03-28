# Test Fixtures

This directory contains test fixture files for integration tests.

## Image Fixtures (Included)

| File | Description | Size |
|------|-------------|------|
| `sample.png` | Basic test image | ~10KB |
| `sample.jpg` | JPEG test image | ~5KB |
| `sample.svg` | Vector test image | <1KB |
| `sample_transparent.png` | PNG with transparency | ~2KB |
| `solid_blue.png` | Solid color test | <1KB |
| `solid_red.png` | Solid color test | <1KB |
| `large.png` | Larger image for perf tests | ~1MB |
| `corrupt/` | Directory with corrupted images for error testing | - |

## Video/Audio Fixtures (Not Included)

Video and audio test fixtures are **not committed** due to file size. They must be downloaded separately.

### Download Script

Run the download script to fetch sample files:

```bash
# From project root
./dpf/scripts/download-test-fixtures.sh

# Or manually
cd dpf/test_fixtures
curl -L -o sample.mp4 "https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4"
curl -L -o sample.mp3 "https://www2.cs.uic.edu/~troy伍testbed/iso-13346/team9/public/BigBuckBunny/video%20and%20audio/BigBuckBunny_512kb.mp3"
```

### Public Sample Sources

| Type | Source | URL |
|------|--------|-----|
| Video (MP4/H264) | Sample-Videos.com | https://www.sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4 |
| Video (MP4/H264) | File Examples | https://file-examples.com/storage/fef1f86049c34ef16e127a7/2017/04/file_example_MP4_480_1_5MG.mp4 |
| Audio (MP3) | File Examples | https://file-examples.com/storage/fef1f86049c34ef16e127a7/2017/11/file_example_MP3_700KB.mp3 |
| Audio (MP3) | GitHub test files | Various public audio test files |

### Manual Download Commands

```bash
# Video sample (Big Buck Bunny - 1MB excerpt)
curl -L -o sample.mp4 \
  "https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4"

# Audio sample (MP3)
curl -L -o sample.mp3 \
  "https://file-examples.com/storage/fef1f86049c34ef16e127a7/2017/11/file_example_MP3_700KB.mp3"
```

### Verify Fixtures

After downloading, verify the files exist:

```bash
ls -la test_fixtures/*.mp4 test_fixtures/*.mp3 2>/dev/null
```

Expected output:
```
sample.mp3  sample.mp4
```

## Running Tests

### All tests (including ignored video/audio):
```bash
cd dpf
cargo test --include-ignored
```

### Only video/audio integration tests:
```bash
cd dpf
cargo test video_ audio_ --include-ignored
```

### Unit tests only (no fixtures required):
```bash
cd dpf
cargo test --lib
```

## Test Categories

1. **Unit tests** (`cargo test --lib`) - No fixtures needed, test parameter parsing and serialization
2. **Integration tests** (`cargo test --test integration_tests`) - Some require fixtures, marked with `#[ignore]`
3. **Ignored integration tests** (`cargo test --include-ignored`) - Require video/audio fixtures

## Adding New Fixtures

1. Download the file to `test_fixtures/`
2. Verify with `file` command:
   ```bash
   file sample.mp4
   # Expected: sample.mp4: ISO Media, MP4 v2
   ```
3. Test with dpf CLI:
   ```bash
   ./target/release/dpf process --job '{"operation":"video_transcode","input":"test_fixtures/sample.mp4","output":"/tmp/test.mp4","codec":"h264"}'
   ```
