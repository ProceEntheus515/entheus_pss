using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using LibVLCSharp.Shared;
using LibVLCSharp.WPF;
using VlcMediaPlayer = LibVLCSharp.Shared.MediaPlayer;

using PerimeterGuard.Configuration;
using PerimeterGuard.Yolo;

namespace Viewer.App;

public partial class MainWindow : Window
{
    private readonly List<GridCell> _cells = new();

    private LibVLC? _libVlc;
    private VlcMediaPlayer? _mediaPlayer;
    private Media? _currentMedia;

    public MainWindow()
    {
        InitializeComponent();
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        try
        {
            _libVlc = new LibVLC(
                enableDebugLogs: true,
                "--avcodec-hw=none");
            _libVlc.Log += OnVlcLog;

            _mediaPlayer = new VlcMediaPlayer(_libVlc);

            _mediaPlayer.Opening += (_, _) => AppendLog("INFO", "Abriendo stream RTSP en LibVLC.");
            _mediaPlayer.Playing += (_, _) => AppendLog("INFO", "Stream RTSP en reproducción en LibVLC.");
            _mediaPlayer.Stopped += (_, _) => AppendLog("INFO", "Stream RTSP detenido en LibVLC.");
            _mediaPlayer.EncounteredError += (_, _) =>
                AppendLog("ERROR", "LibVLC reportó un error al reproducir el stream RTSP.");

            AppendLog("INFO", "LibVLC inicializado correctamente.");
        }
        catch (Exception ex)
        {
            AppendLog("ERROR", $"Error al inicializar LibVLC: {ex.Message}");
        }

        BuildLayoutFromSelection();
    }

    private void OnVlcLog(object? sender, LogEventArgs e)
    {
        var level = e.Level switch
        {
            LogLevel.Debug => "VLC-DEBUG",
            LogLevel.Notice => "VLC-NOTICE",
            LogLevel.Warning => "VLC-WARN",
            LogLevel.Error => "VLC-ERROR",
            _ => "VLC",
        };

        AppendLog(level, e.Message);
        Console.WriteLine($"[{level}] {e.Message}");
    }

    private void AppendLog(string level, string message)
    {
        if (!Dispatcher.CheckAccess())
        {
            Dispatcher.Invoke(() => AppendLog(level, message));
            return;
        }

        var timestamp = DateTime.Now.ToString("HH:mm:ss");
        var line = $"{timestamp} [{level}] {message}";

        LogTextBox.AppendText(line + Environment.NewLine);
        LogTextBox.ScrollToEnd();
    }

    private void OnLayoutSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!IsLoaded)
        {
            return;
        }

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

    private void OnBuildSmartPssUrlClick(object sender, RoutedEventArgs e)
    {
        var selectedSystem = (SystemSelector.SelectedItem as ComboBoxItem)?.Content?.ToString() ?? string.Empty;

        if (!selectedSystem.StartsWith("SmartPSS", StringComparison.OrdinalIgnoreCase))
        {
            AppendLog("INFO", "Sistema seleccionado distinto de SmartPSS; por ahora solo se soporta SmartPSS.");

            MessageBox.Show(
                "Por ahora solo está soportado el constructor de enlaces SmartPSS.",
                "Constructor de enlaces",
                MessageBoxButton.OK,
                MessageBoxImage.Information);

            return;
        }

        var user = RtspUserTextBox.Text?.Trim() ?? string.Empty;
        var password = RtspPasswordBox.Password ?? string.Empty;
        var host = RtspHostTextBox.Text?.Trim() ?? string.Empty;
        var portText = RtspPortTextBox.Text?.Trim() ?? "554";

        if (string.IsNullOrWhiteSpace(user) ||
            string.IsNullOrWhiteSpace(password) ||
            string.IsNullOrWhiteSpace(host) ||
            string.IsNullOrWhiteSpace(portText))
        {
            AppendLog("WARN", "Intento de construir enlace RTSP con campos incompletos.");

            MessageBox.Show(
                "Debe completar usuario, contraseña, host y puerto para construir el enlace RTSP.",
                "Datos incompletos",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);

            return;
        }

        if (!int.TryParse(portText, out var port) || port <= 0 || port > 65535)
        {
            AppendLog("WARN", $"Puerto inválido especificado: '{portText}'.");

            MessageBox.Show(
                "El puerto especificado no es válido.",
                "Puerto inválido",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);

            return;
        }

        var encodedPassword = Uri.EscapeDataString(password);
        var rtspUrl =
            $"rtsp://{user}:{encodedPassword}@{host}:{port}/cam/realmonitor?channel=1&subtype=0";

        AppendLog("INFO", $"Enlace RTSP tipo SmartPSS construido: {rtspUrl}");

        if (!TryStartRtspPlayback(rtspUrl))
        {
            MessageBox.Show(
                rtspUrl,
                "Enlace RTSP tipo SmartPSS (error al reproducir, ver logs)",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);
        }
        else
        {
            MessageBox.Show(
                rtspUrl,
                "Enlace RTSP tipo SmartPSS (reproducción iniciada)",
                MessageBoxButton.OK,
                MessageBoxImage.Information);
        }
    }

    private bool TryStartRtspPlayback(string source)
    {
        if (_libVlc is null || _mediaPlayer is null)
        {
            AppendLog("ERROR", "LibVLC no está inicializado; no se puede iniciar la reproducción RTSP.");
            return false;
        }

        try
        {
            AppendLog("INFO", $"Intentando reproducir fuente: {source}");

            var isLocation = Uri.TryCreate(source, UriKind.Absolute, out var uri) &&
                             (uri.Scheme is "rtsp" or "rtmp" or "http" or "https");

            _currentMedia?.Dispose();
            _currentMedia = isLocation
                ? new Media(_libVlc, source, FromType.FromLocation)
                : new Media(_libVlc, source, FromType.FromPath);

            var started = _mediaPlayer.Play(_currentMedia);

            if (!started)
            {
                AppendLog("ERROR", "MediaPlayer.Play devolvió false al intentar iniciar el stream RTSP.");
            }

            if (started)
            {
                var debugWindow = new VideoDebugWindow(_mediaPlayer);
                debugWindow.Owner = this;
                debugWindow.Show();
            }

            return started;
        }
        catch (Exception ex)
        {
            AppendLog("ERROR", $"Excepción al iniciar reproducción RTSP: {ex.Message}");
            return false;
        }
    }

    private void BuildLayoutFromSelection()
    {
        var selectedLayout = (LayoutSelector.SelectedItem as ComboBoxItem)?.Content?.ToString() ?? "1x1";

        var layout = selectedLayout switch
        {
            "1x1" => new LayoutConfig { Id = "1x1", Rows = 1, Columns = 1 },
            "2x2" => new LayoutConfig { Id = "2x2", Rows = 2, Columns = 2 },
            "3x3" => new LayoutConfig { Id = "3x3", Rows = 3, Columns = 3 },
            "4x4" => new LayoutConfig { Id = "4x4", Rows = 4, Columns = 4 },
            "4x5" => new LayoutConfig { Id = "4x5", Rows = 4, Columns = 5 },
            "5x4" => new LayoutConfig { Id = "5x4", Rows = 5, Columns = 4 },
            _ => new LayoutConfig { Id = "1x1", Rows = 1, Columns = 1 },
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

        var isSingleCellLayout = layout.Rows == 1 && layout.Columns == 1;
        var videoAttached = false;

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

            var shouldAttachVideo = _mediaPlayer is not null && (!videoAttached && isSingleCellLayout || !isSingleCellLayout && !videoAttached);

            if (shouldAttachVideo)
            {
                var container = new Grid();

                var videoView = new VideoView
                {
                    MediaPlayer = _mediaPlayer,
                    Background = Brushes.Black,
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    VerticalAlignment = VerticalAlignment.Stretch,
                };

                container.Children.Add(videoView);
                container.Children.Add(text);

                border.Child = container;
                videoAttached = true;
            }
            else
            {
                border.Child = text;
            }

            CellsGrid.Children.Add(border);
        }
    }
}