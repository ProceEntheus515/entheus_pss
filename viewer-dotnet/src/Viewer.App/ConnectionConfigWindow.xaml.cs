using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using PerimeterGuard.Configuration;

namespace Viewer.App;

public partial class ConnectionConfigWindow : Window
{
    public IReadOnlyList<CameraConfig> Cameras { get; private set; } = Array.Empty<CameraConfig>();
    public string ProfileName { get; private set; } = string.Empty;

    public ConnectionConfigWindow()
    {
        InitializeComponent();
        AddCameraRow();
    }

    private void OnAddCameraClick(object sender, RoutedEventArgs e)
    {
        AddCameraRow();
    }

    private void AddCameraRow()
    {
        var row = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Margin = new Thickness(0, 2, 0, 2),
        };

        var hostBox = new TextBox
        {
            Width = 260,
            Margin = new Thickness(0, 0, 8, 0),
            ToolTip = "IP / host",
        };

        var channelBox = new TextBox
        {
            Width = 60,
            Margin = new Thickness(0, 0, 8, 0),
            Text = "1",
        };

        var removeButton = new Button
        {
            Content = "Eliminar",
            Width = 70,
        };

        removeButton.Click += (_, _) =>
        {
            CamerasPanel.Children.Remove(row);
        };

        row.Children.Add(hostBox);
        row.Children.Add(channelBox);
        row.Children.Add(removeButton);

        CamerasPanel.Children.Add(row);
    }

    private void OnSaveClick(object sender, RoutedEventArgs e)
    {
        var profileName = ProfileNameTextBox.Text?.Trim() ?? string.Empty;
        var user = UserTextBox.Text?.Trim() ?? string.Empty;
        var password = PasswordBox.Password ?? string.Empty;
        var portText = PortTextBox.Text?.Trim() ?? "554";

        if (string.IsNullOrWhiteSpace(profileName) ||
            string.IsNullOrWhiteSpace(user) ||
            string.IsNullOrWhiteSpace(password) ||
            string.IsNullOrWhiteSpace(portText))
        {
            MessageBox.Show(
                "Debe completar nombre de perfil, usuario, contraseña y puerto, y definir al menos una cámara.",
                "Datos incompletos",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);
            return;
        }

        if (!int.TryParse(portText, out var port) || port <= 0 || port > 65535)
        {
            MessageBox.Show(
                "El puerto especificado no es válido.",
                "Puerto inválido",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);
            return;
        }

        var encodedPassword = Uri.EscapeDataString(password);

        var cameraConfigs = new List<CameraConfig>();

        foreach (var child in CamerasPanel.Children.OfType<StackPanel>())
        {
            var textBoxes = child.Children.OfType<TextBox>().ToList();
            if (textBoxes.Count < 2)
            {
                continue;
            }

            var host = textBoxes[0].Text?.Trim() ?? string.Empty;
            var channelText = textBoxes[1].Text?.Trim() ?? string.Empty;

            if (string.IsNullOrWhiteSpace(host) || string.IsNullOrWhiteSpace(channelText))
            {
                continue;
            }

            if (!int.TryParse(channelText, out var channel) || channel <= 0)
            {
                MessageBox.Show(
                    $"Channel inválido para host '{host}'. Debe ser un entero mayor que cero.",
                    "Channel inválido",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
                return;
            }

            cameraConfigs.Add(
                new CameraConfig
                {
                    Id = $"cam_{host}_{channel}",
                    Name = $"Cam {host} ch{channel}",
                    RtspUrl =
                        $"rtsp://{user}:{encodedPassword}@{host}:{port}/cam/realmonitor?channel={channel}&subtype=0",
                    Enabled = true,
                });
        }

        if (cameraConfigs.Count == 0)
        {
            MessageBox.Show(
                "Debe definir al menos una cámara con IP y channel válidos.",
                "Cámaras inválidas",
                MessageBoxButton.OK,
                MessageBoxImage.Warning);
            return;
        }

        Cameras = cameraConfigs;
        ProfileName = profileName;
        DialogResult = true;
        Close();
    }
}

