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
    private readonly List<ConnectionProfile> _profiles = new();
    private List<CameraConfig> _configuredCameras = new();

    private sealed class CameraView
    {
        public CameraConfig Camera { get; init; } = null!;
        public VlcMediaPlayer Player { get; init; } = null!;
        public Media? Media { get; set; }
        public VideoView View { get; init; } = null!;
    }

    private readonly List<CameraView> _cameraViews = new();

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

            AppendLog("INFO", "LibVLC inicializado correctamente.");

            LoadProfiles();
        }
        catch (Exception ex)
        {
            AppendLog("ERROR", $"Error al inicializar LibVLC: {ex.Message}");
        }

        BuildLayoutFromSelection();
    }

    private void LoadProfiles()
    {
        _profiles.Clear();
        _profiles.AddRange(ConnectionProfilesStore.Load());

        ProfileComboBox.ItemsSource = null;
        ProfileComboBox.ItemsSource = _profiles;

        if (_profiles.Count > 0)
        {
            ProfileComboBox.SelectedIndex = 0;
        }
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
        var dialog = new ConnectionConfigWindow
        {
            Owner = this,
        };

        var result = dialog.ShowDialog();

        if (result != true || dialog.Cameras.Count == 0)
        {
            return;
        }

        var profileName = dialog.ProfileName;
        var existing = _profiles.FirstOrDefault(p =>
            string.Equals(p.Name, profileName, StringComparison.OrdinalIgnoreCase));

        if (existing is null)
        {
            existing = new ConnectionProfile
            {
                Name = profileName,
                Cameras = dialog.Cameras.ToList(),
            };
            _profiles.Add(existing);
        }
        else
        {
            existing.Cameras = dialog.Cameras.ToList();
        }

        ConnectionProfilesStore.Save(_profiles);
        LoadProfiles();

        ProfileComboBox.SelectedItem = existing;

        _configuredCameras = existing.Cameras;

        AppendLog("INFO", $"Configuración RTSP guardada en perfil '{existing.Name}' con {_configuredCameras.Count} cámaras.");

        StartRtspForConfiguredCameras();
    }

    private void OnConnectProfileClick(object sender, RoutedEventArgs e)
    {
        if (ProfileComboBox.SelectedItem is not ConnectionProfile profile ||
            profile.Cameras.Count == 0)
        {
            AppendLog("WARN", "No hay perfil seleccionado o el perfil no tiene cámaras configuradas.");
            return;
        }

        _configuredCameras = profile.Cameras;

        AppendLog("INFO", $"Conectando perfil '{profile.Name}' con {_configuredCameras.Count} cámaras.");

        StartRtspForConfiguredCameras();
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

        var cameras = _configuredCameras.Count > 0
            ? _configuredCameras
            : BuildSampleCameras(layout);

        _cells.Clear();
        _cells.AddRange(GridLayoutManager.BuildGrid(layout, cameras));

        RenderGrid(layout);
    }

    private void StartRtspForConfiguredCameras()
    {
        if (_libVlc is null)
        {
            AppendLog("ERROR", "LibVLC no está inicializado; no se puede iniciar la reproducción RTSP.");
            return;
        }

        foreach (var view in _cameraViews)
        {
            try
            {
                view.Player.Stop();
                view.Media?.Dispose();
                view.Player.Dispose();
            }
            catch
            {
                // Ignoramos errores de limpieza en esta fase
            }
        }

        _cameraViews.Clear();

        foreach (var camera in _configuredCameras)
        {
            try
            {
                AppendLog("INFO", $"Iniciando reproducción RTSP para cámara '{camera.Name}' ({camera.RtspUrl})");

                var player = new VlcMediaPlayer(_libVlc);

                player.Opening += (_, _) => AppendLog("INFO", $"[{camera.Name}] Abriendo stream RTSP.");
                player.Playing += (_, _) => AppendLog("INFO", $"[{camera.Name}] Stream RTSP en reproducción.");
                player.Stopped += (_, _) => AppendLog("INFO", $"[{camera.Name}] Stream RTSP detenido.");
                player.EncounteredError += (_, _) =>
                    AppendLog("ERROR", $"[{camera.Name}] LibVLC reportó un error al reproducir el stream RTSP.");

                var media = new Media(_libVlc, camera.RtspUrl, FromType.FromLocation);

                var view = new VideoView
                {
                    MediaPlayer = player,
                    Background = Brushes.Black,
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    VerticalAlignment = VerticalAlignment.Stretch,
                };

                _cameraViews.Add(
                    new CameraView
                    {
                        Camera = camera,
                        Player = player,
                        Media = media,
                        View = view,
                    });
            }
            catch (Exception ex)
            {
                AppendLog("ERROR", $"Excepción al iniciar reproducción RTSP para cámara '{camera.Name}': {ex.Message}");
            }
        }

        BuildLayoutFromSelection();
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
                Margin = new Thickness(1),
            };

            var text = new TextBlock
            {
                Text = $"{cell.CellId}\n{cell.CameraId}",
                Foreground = Brushes.White,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                TextAlignment = TextAlignment.Center,
            };

            var cameraForCell = _configuredCameras.FirstOrDefault(c => string.Equals(c.Id, cell.CameraId, StringComparison.OrdinalIgnoreCase));
            var cameraView = cameraForCell is null
                ? null
                : _cameraViews.FirstOrDefault(v => string.Equals(v.Camera.Id, cameraForCell.Id, StringComparison.OrdinalIgnoreCase));

            if (cameraView is not null)
            {
                var container = new Grid();
                if (cameraView.View.Parent is Panel currentParentPanel)
                {
                    currentParentPanel.Children.Remove(cameraView.View);
                }

                container.Children.Add(cameraView.View);
                container.Children.Add(text);

                border.Child = container;

                if (cameraView.Media is not null && !cameraView.Player.IsPlaying)
                {
                    var started = cameraView.Player.Play(cameraView.Media);
                    if (!started)
                    {
                        AppendLog("ERROR", $"[{cameraView.Camera.Name}] MediaPlayer.Play devolvió false al intentar iniciar el stream RTSP.");
                    }
                }
            }
            else
            {
                border.Child = text;
            }

            CellsGrid.Children.Add(border);
        }
    }
}