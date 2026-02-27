using System.Text.Json.Serialization;

namespace PerimeterGuard.Yolo.Contracts;

/// <summary>
/// Representa una caja delimitadora en coordenadas de imagen.
/// </summary>
public sealed class BoundingBoxDto
{
    [JsonPropertyName("x_min")]
    public int XMin { get; set; }

    [JsonPropertyName("y_min")]
    public int YMin { get; set; }

    [JsonPropertyName("x_max")]
    public int XMax { get; set; }

    [JsonPropertyName("y_max")]
    public int YMax { get; set; }
}

/// <summary>
/// Representa una detección individual devuelta por YOLO.
/// </summary>
public sealed class DetectionDto
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;

    [JsonPropertyName("confidence")]
    public float Confidence { get; set; }

    [JsonPropertyName("bbox")]
    public BoundingBoxDto Bbox { get; set; } = new();
}

/// <summary>
/// Frame individual que se envía al servicio YOLO.
/// </summary>
public sealed class InferFrameRequestDto
{
    [JsonPropertyName("frame_id")]
    public string FrameId { get; set; } = string.Empty;

    [JsonPropertyName("camera_id")]
    public string CameraId { get; set; } = string.Empty;

    [JsonPropertyName("cell_id")]
    public string CellId { get; set; } = string.Empty;

    [JsonPropertyName("width")]
    public int Width { get; set; }

    [JsonPropertyName("height")]
    public int Height { get; set; }

    [JsonPropertyName("image_format")]
    public string ImageFormat { get; set; } = "jpg";

    [JsonPropertyName("image_base64")]
    public string ImageBase64 { get; set; } = string.Empty;
}

/// <summary>
/// Opciones de inferencia que acompañan a la petición.
/// </summary>
public sealed class InferOptionsDto
{
    [JsonPropertyName("confidence_threshold")]
    public float ConfidenceThreshold { get; set; } = 0.5f;

    [JsonPropertyName("iou_threshold")]
    public float IouThreshold { get; set; } = 0.45f;

    [JsonPropertyName("max_detections_per_frame")]
    public int MaxDetectionsPerFrame { get; set; } = 50;

    [JsonPropertyName("classes")]
    public List<string> Classes { get; set; } = new() { "person" };
}

/// <summary>
/// Petición de inferencia batch enviada al backend YOLO.
/// </summary>
public sealed class InferRequestDto
{
    [JsonPropertyName("request_id")]
    public string RequestId { get; set; } = string.Empty;

    [JsonPropertyName("timestamp_utc")]
    public string TimestampUtc { get; set; } = string.Empty;

    [JsonPropertyName("frames")]
    public List<InferFrameRequestDto> Frames { get; set; } = new();

    [JsonPropertyName("options")]
    public InferOptionsDto Options { get; set; } = new();
}

/// <summary>
/// Respuesta por frame devuelta por el backend YOLO.
/// </summary>
public sealed class InferFrameResponseDto
{
    [JsonPropertyName("frame_id")]
    public string FrameId { get; set; } = string.Empty;

    [JsonPropertyName("camera_id")]
    public string CameraId { get; set; } = string.Empty;

    [JsonPropertyName("detections")]
    public List<DetectionDto> Detections { get; set; } = new();
}

/// <summary>
/// Respuesta global de una petición de inferencia batch.
/// </summary>
public sealed class InferResponseDto
{
    [JsonPropertyName("request_id")]
    public string RequestId { get; set; } = string.Empty;

    [JsonPropertyName("timestamp_utc")]
    public string TimestampUtc { get; set; } = string.Empty;

    [JsonPropertyName("processing_time_ms")]
    public int ProcessingTimeMs { get; set; }

    [JsonPropertyName("frames")]
    public List<InferFrameResponseDto> Frames { get; set; } = new();
}

