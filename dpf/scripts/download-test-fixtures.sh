#!/bin/bash
# Setup test fixtures for video and audio integration tests.
# Run from project root: ./dpf/scripts/download-test-fixtures.sh
# Or from dpf directory: ./scripts/download-test-fixtures.sh

set -e

FIXTURES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../test_fixtures" && pwd)"
echo "Setting up test fixtures in: $FIXTURES_DIR"
cd "$FIXTURES_DIR"

# Track status
VIDEO_DONE=false
AUDIO_DONE=false

# ─── Video Sample ──────────────────────────────────────────────────
echo "Setting up video sample..."
if [ -f sample.mp4 ] && file sample.mp4 | grep -q "ISO Media"; then
    echo "  ✓ sample.mp4 already exists ($(du -h sample.mp4 | cut -f1))"
    VIDEO_DONE=true
elif command -v ffmpeg &> /dev/null; then
    echo "  Generating test video (5s, 480x360, H.264/AAC) with FFmpeg..."
    ffmpeg -f lavfi -i testsrc=duration=5:size=480x360:rate=30 \
           -f lavfi -i sine=frequency=1000:duration=5 \
           -c:v libx264 -preset ultrafast -c:a aac \
           sample.mp4 -y 2>/dev/null
    echo "  ✓ sample.mp4 generated ($(du -h sample.mp4 | cut -f1))"
    VIDEO_DONE=true
else
    echo "  ✗ FFmpeg not found - cannot generate video"
fi

# ─── Audio Sample ──────────────────────────────────────────────────
echo "Setting up audio sample..."
if [ -f sample.mp3 ] && file sample.mp3 | grep -q "MP3"; then
    echo "  ✓ sample.mp3 already exists ($(du -h sample.mp3 | cut -f1))"
    AUDIO_DONE=true
elif command -v ffmpeg &> /dev/null; then
    echo "  Generating test audio (10s, 440Hz sine wave) with FFmpeg..."
    ffmpeg -f lavfi -i "sine=frequency=440:duration=10" \
           -c:a libmp3lame -b:a 128k \
           sample.mp3 -y 2>/dev/null
    echo "  ✓ sample.mp3 generated ($(du -h sample.mp3 | cut -f1))"
    AUDIO_DONE=true
else
    echo "  ✗ FFmpeg not found - cannot generate audio"
fi

# ─── Summary ───────────────────────────────────────────────────────
echo ""
echo "=== Fixture Summary ==="
if [ "$VIDEO_DONE" = true ]; then
    echo "Video:   ✓ sample.mp4 ($(du -h sample.mp4 | cut -f1))"
else
    echo "Video:   ✗ Not available"
fi

if [ "$AUDIO_DONE" = true ]; then
    echo "Audio:   ✓ sample.mp3 ($(du -h sample.mp3 | cut -f1))"
else
    echo "Audio:   ✗ Not available"
fi

echo ""
echo "Run integration tests:"
echo "  cd dpf && cargo test -- --include-ignored"
