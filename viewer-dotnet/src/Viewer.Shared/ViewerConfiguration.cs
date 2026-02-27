namespace PerimeterGuard.Configuration;

/// <summary>
/// Configuración de una cámara individual.
/// </summary>
public sealed class CameraConfig
{
    public string Id { get; set; } = string.Empty;

    public string Name { get; set; } = string.Empty;

    public string RtspUrl { get; set; } = string.Empty;

    public bool Enabled { get; set; } = true;
}

/// <summary>
/// Configuración de layout de grid (filas, columnas, etiqueta).
/// </summary>
public sealed class LayoutConfig
{
    public string Id { get; set; } = "2x2";

    public int Rows { get; set; } = 2;

    public int Columns { get; set; } = 2;
}

/// <summary>
/// Configuración de zona de intrusión asociada a una cámara o celda.
/// En esta fase solo se define la estructura, sin lógica compleja.
/// </summary>
public sealed class ZoneConfig
{
    public string Id { get; set; } = string.Empty;

    public string CameraId { get; set; } = string.Empty;

    public string Type { get; set; } = "rectangle";

    /// <summary>
    /// Puntos normalizados (0-1) respecto al ancho/alto de la imagen.
    /// </summary>
    public List<(double X, double Y)> Points { get; set; } = new();
}

/// <summary>
/// Agrega la configuración general de la aplicación.
/// </summary>
public sealed class AppSettings
{
    public List<CameraConfig> Cameras { get; set; } = new();

    public List<LayoutConfig> Layouts { get; set; } = new();

    public List<ZoneConfig> Zones { get; set; } = new();
}

/// <summary>
/// Representa una celda del grid visual (fila, columna, cámara asociada).
/// </summary>
public sealed class GridCell
{
    public int RowIndex { get; set; }

    public int ColumnIndex { get; set; }

    public string CameraId { get; set; } = string.Empty;

    public string CellId { get; set; } = string.Empty;
}

/// <summary>
/// Utilidad para generar celdas de grid a partir de un layout y una lista de cámaras.
/// </summary>
public static class GridLayoutManager
{
    public static List<GridCell> BuildGrid(LayoutConfig layout, IReadOnlyList<CameraConfig> cameras)
    {
        var cells = new List<GridCell>();

        if (layout.Rows <= 0 || layout.Columns <= 0)
        {
            return cells;
        }

        var totalCells = layout.Rows * layout.Columns;

        for (var index = 0; index < totalCells; index++)
        {
            var row = index / layout.Columns;
            var column = index % layout.Columns;

            var camera = index < cameras.Count ? cameras[index] : null;

            var cell = new GridCell
            {
                RowIndex = row,
                ColumnIndex = column,
                CameraId = camera?.Id ?? $"cam_{index}",
                CellId = $"{layout.Id}_{row}_{column}",
            };

            cells.Add(cell);
        }

        return cells;
    }
}

