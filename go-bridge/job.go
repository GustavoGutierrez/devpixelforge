package dpf

// ─── Crop Job ─────────────────────────────────────────────────────

// CropRect defines a rectangular region for manual crop.
type CropRect struct {
	X      uint32 `json:"x"`
	Y      uint32 `json:"y"`
	Width  uint32 `json:"width"`
	Height uint32 `json:"height"`
}

// CropJob defines a crop operation.
type CropJob struct {
	Operation string    `json:"operation"`
	Input     string    `json:"input"`
	Output    string    `json:"output"`
	Rect      *CropRect `json:"rect,omitempty"`
	Gravity   string    `json:"gravity,omitempty"`
	FocalX    *float64  `json:"focal_x,omitempty"`
	FocalY    *float64  `json:"focal_y,omitempty"`
	Width     *uint32   `json:"width,omitempty"`
	Height    *uint32   `json:"height,omitempty"`
	Format    string    `json:"format,omitempty"`
	Quality   *uint8    `json:"quality,omitempty"`
	Inline    bool      `json:"inline,omitempty"`
}

// ─── Rotate Job ───────────────────────────────────────────────────

// RotateJob defines a rotate operation.
type RotateJob struct {
	Operation  string   `json:"operation"`
	Input      string   `json:"input"`
	Output     string   `json:"output"`
	Angle      *uint16  `json:"angle,omitempty"`
	AngleF     *float64 `json:"angle_f,omitempty"`
	Flip       string   `json:"flip,omitempty"`
	AutoOrient bool     `json:"auto_orient,omitempty"`
	Background string   `json:"background,omitempty"`
	Format     string   `json:"format,omitempty"`
	Quality    *uint8   `json:"quality,omitempty"`
	Inline     bool     `json:"inline,omitempty"`
}

// ─── Watermark Job ───────────────────────────────────────────────

// WatermarkJob defines a watermark operation.
type WatermarkJob struct {
	Operation string   `json:"operation"`
	Input     string   `json:"input"`
	Output    string   `json:"output"`
	Text      string   `json:"text,omitempty"`
	Image     string   `json:"image,omitempty"`
	Position  string   `json:"position,omitempty"`
	Opacity   *float64 `json:"opacity,omitempty"`
	FontSize  *uint32  `json:"font_size,omitempty"`
	Color     string   `json:"color,omitempty"`
	OffsetX   *uint32  `json:"offset_x,omitempty"`
	OffsetY   *uint32  `json:"offset_y,omitempty"`
	Format    string   `json:"format,omitempty"`
	Quality   *uint8   `json:"quality,omitempty"`
	Inline    bool     `json:"inline,omitempty"`
}

// ─── Adjust Job ──────────────────────────────────────────────────

// AdjustJob defines an image adjustment operation.
type AdjustJob struct {
	Operation  string   `json:"operation"`
	Input      string   `json:"input"`
	Output     string   `json:"output"`
	Brightness *float64 `json:"brightness,omitempty"`
	Contrast   *float64 `json:"contrast,omitempty"`
	Saturation *float64 `json:"saturation,omitempty"`
	Blur       *float64 `json:"blur,omitempty"`
	Sharpen    *float64 `json:"sharpen,omitempty"`
	LinearRGB  bool     `json:"linear_rgb,omitempty"`
	Format     string   `json:"format,omitempty"`
	Quality    *uint8   `json:"quality,omitempty"`
	Inline     bool     `json:"inline,omitempty"`
}

// ─── Quality Job ─────────────────────────────────────────────────

// QualityJob defines an auto-quality optimization operation.
type QualityJob struct {
	Operation        string   `json:"operation"`
	Input            string   `json:"input"`
	Output           string   `json:"output"`
	TargetSize       uint64   `json:"target_size"`
	TolerancePercent *float64 `json:"tolerance_percent,omitempty"`
	MaxIterations    *uint8   `json:"max_iterations,omitempty"`
	MinQuality       *uint8   `json:"min_quality,omitempty"`
	MaxQuality       *uint8   `json:"max_quality,omitempty"`
	Format           string   `json:"format"`
	Inline           bool     `json:"inline,omitempty"`
}

// ─── Srcset Job ──────────────────────────────────────────────────

// SrcsetJob defines a responsive srcset generation operation.
type SrcsetJob struct {
	Operation    string    `json:"operation"`
	Input        string    `json:"input"`
	OutputDir    string    `json:"output_dir"`
	Widths       []uint32  `json:"widths"`
	Densities    []float64 `json:"densities,omitempty"`
	Format       string    `json:"format,omitempty"`
	Quality      *uint8    `json:"quality,omitempty"`
	GenerateHTML bool      `json:"generate_html,omitempty"`
	LinearRGB    bool      `json:"linear_rgb,omitempty"`
}

// ─── Exif Job ────────────────────────────────────────────────────

// ExifJob defines an EXIF metadata operation.
type ExifJob struct {
	Operation      string   `json:"operation"`
	Input          string   `json:"input"`
	Output         string   `json:"output,omitempty"`
	ExifOp         string   `json:"exif_op"`
	Mode           string   `json:"mode,omitempty"`
	Keep           []string `json:"keep,omitempty"`
	ReturnMetadata bool     `json:"return_metadata,omitempty"`
	Format         string   `json:"format,omitempty"`
	Quality        *uint8   `json:"quality,omitempty"`
	Inline         bool     `json:"inline,omitempty"`
}
