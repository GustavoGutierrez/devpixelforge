package dpf

// ─── Video Jobs ────────────────────────────────────────────────────

// VideoTranscodeJob - Transcode video to different codec
type VideoTranscodeJob struct {
	Operation  string `json:"operation"`
	Input      string `json:"input"`
	Output     string `json:"output"`
	Codec      string `json:"codec,omitempty"`   // h264, h265, vp8, vp9, av1
	Bitrate    string `json:"bitrate,omitempty"` // e.g., "2M", "5000k"
	Preset     string `json:"preset,omitempty"`  // ultrafast, fast, medium, slow, veryslow
	CRF        *uint8 `json:"crf,omitempty"`     // 0-51
	AudioCodec string `json:"audio_codec,omitempty"`
}

// VideoResizeJob - Resize video dimensions
type VideoResizeJob struct {
	Operation      string `json:"operation"`
	Input          string `json:"input"`
	Output         string `json:"output"`
	Width          uint32 `json:"width,omitempty"`
	Height         uint32 `json:"height,omitempty"`
	MaintainAspect bool   `json:"maintain_aspect,omitempty"`
}

// VideoTrimJob - Trim video by timestamps
type VideoTrimJob struct {
	Operation string  `json:"operation"`
	Input     string  `json:"input"`
	Output    string  `json:"output"`
	Start     float64 `json:"start"` // seconds
	End       float64 `json:"end"`   // seconds
}

// VideoThumbnailJob - Extract frames as images
type VideoThumbnailJob struct {
	Operation string `json:"operation"`
	Input     string `json:"input"`
	Output    string `json:"output"`
	Timestamp string `json:"timestamp"`        // "25%" or seconds like "30.5"
	Format    string `json:"format,omitempty"` // jpeg, png, webp
	Quality   *uint8 `json:"quality,omitempty"`
}

// VideoProfileJob - Apply encoding profile
type VideoProfileJob struct {
	Operation string `json:"operation"`
	Input     string `json:"input"`
	Output    string `json:"output"`
	Profile   string `json:"profile"` // web-low, web-mid, web-high
}

// VideoMetadataJob - Extract video metadata
type VideoMetadataJob struct {
	Operation string `json:"operation"`
	Input     string `json:"input"`
}
