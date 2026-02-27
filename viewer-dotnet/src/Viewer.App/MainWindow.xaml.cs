using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

using PerimeterGuard.Configuration;
using PerimeterGuard.Yolo;

namespace Viewer.App;

public partial class MainWindow : Window
{
    private readonly List<GridCell> _cells = new();

    public MainWindow()
    {
        InitializeComponent();
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        BuildLayoutFromSelection();
    }

    private void OnGenerateJsonClick(object sender, RoutedEventArgs e)
    {
        if (_cells.Count == 0)
        {
            BuildLayoutFromSelection();
        }

        var json = YoloRequestBuilder.BuildSampleRequestJson(_cells);

        MessageBox.Show(
            json,
            "Payload de ejemplo para /infer",
            MessageBoxButton.OK,
            MessageBoxImage.Information);
    }

    private void BuildLayoutFromSelection()
    {
        var selectedLayout = (LayoutSelector.SelectedItem as ComboBoxItem)?.Content?.ToString() ?? "2x2";

        var layout = selectedLayout switch
        {
            "3x3" => new LayoutConfig { Id = "3x3", Rows = 3, Columns = 3 },
            _ => new LayoutConfig { Id = "2x2", Rows = 2, Columns = 2 },
        };

        var cameras = BuildSampleCameras(layout);

        _cells.Clear();
        _cells.AddRange(GridLayoutManager.BuildGrid(layout, cameras));

        RenderGrid(layout);
    }

    private static List<CameraConfig> BuildSampleCameras(LayoutConfig layout)
    {
        var totalCells = Math.Max(layout.Rows * layout.Columns, 1);
        var cameras = new List<CameraConfig>();

        for (var i = 0; i < totalCells; i++)
        {
            cameras.Add(
                new CameraConfig
                {
                    Id = $"cam_{i + 1}",
                    Name = $"Cámara {i + 1}",
                    RtspUrl = string.Empty,
                    Enabled = true,
                });
        }

        return cameras;
    }

    private void RenderGrid(LayoutConfig layout)
    {
        CellsGrid.Children.Clear();
        CellsGrid.Rows = layout.Rows;
        CellsGrid.Columns = layout.Columns;

        foreach (var cell in _cells)
        {
            var border = new Border
            {
                Background = new SolidColorBrush(Color.FromRgb(30, 60, 110)),
                BorderBrush = Brushes.White,
                BorderThickness = new Thickness(1),
                Margin = new Thickness(4),
            };

            var text = new TextBlock
            {
                Text = $"{cell.CellId}\n{cell.CameraId}",
                Foreground = Brushes.White,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                TextAlignment = TextAlignment.Center,
            };

            border.Child = text;
            CellsGrid.Children.Add(border);
        }
    }
}