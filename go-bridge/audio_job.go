package dpf

// ─── Audio Jobs ────────────────────────────────────────────────────

// AudioTranscodeJob - Transcode audio format
type AudioTranscodeJob struct {
	Operation  string  `json:"operation"`
	Input      string  `json:"input"`
	Output     string  `json:"output"`
	Codec      string  `json:"codec,omitempty"`   // mp3, aac, opus, vorbis, flac, wav
	Bitrate    string  `json:"bitrate,omitempty"` // e.g., "192k"
	SampleRate *uint32 `json:"sample_rate,omitempty"`
	Channels   *uint32 `json:"channels,omitempty"` // 1 = mono, 2 = stereo
	Quality    *uint8  `json:"quality,omitempty"`
}

// AudioTrimJob - Trim audio by timestamps
type AudioTrimJob struct {
	Operation string  `json:"operation"`
	Input     string  `json:"input"`
	Output    string  `json:"output"`
	Start     float64 `json:"start"` // seconds
	End       float64 `json:"end"`   // seconds
}

// AudioNormalizeJob - Loudness normalization
type AudioNormalizeJob struct {
	Operation  string   `json:"operation"`
	Input      string   `json:"input"`
	Output     string   `json:"output"`
	TargetLUFS float64  `json:"target_lufs"` // e.g., -14.0
	Threshold  *float64 `json:"threshold_lufs,omitempty"`
}

// AudioSilenceTrimJob - Remove silence from audio
type AudioSilenceTrimJob struct {
	Operation   string   `json:"operation"`
	Input       string   `json:"input"`
	Output      string   `json:"output"`
	ThresholdDB *float64 `json:"threshold_db,omitempty"` // default -40
	MinDuration *float64 `json:"min_duration,omitempty"` // default 0.5
}
