using System.Text.Json;
using PerimeterGuard.Configuration;
using PerimeterGuard.Yolo.Contracts;

namespace PerimeterGuard.Yolo;

/// <summary>
/// Construye peticiones de prueba para el backend YOLO
/// a partir de la disposición actual de celdas del grid.
/// </summary>
public static class YoloRequestBuilder
{
    public static InferRequestDto BuildSampleRequest(IEnumerable<GridCell> cells)
    {
        var request = new InferRequestDto
        {
            RequestId = Guid.NewGuid().ToString(),
            TimestampUtc = DateTime.UtcNow.ToString("O"),
            Options = new InferOptionsDto(),
        };

        foreach (var cell in cells)
        {
            var frame = new InferFrameRequestDto
            {
                FrameId = $"frame-{cell.CellId}",
                CameraId = cell.CameraId,
                CellId = cell.CellId,
                Width = 1280,
                Height = 720,
                ImageFormat = "jpg",
                ImageBase64 = string.Empty,
            };

            request.Frames.Add(frame);
        }

        return request;
    }

    public static string BuildSampleRequestJson(IEnumerable<GridCell> cells)
    {
        var dto = BuildSampleRequest(cells);

        var json = JsonSerializer.Serialize(
            dto,
            new JsonSerializerOptions
            {
                WriteIndented = true,
            }
        );

        return json;
    }
}

