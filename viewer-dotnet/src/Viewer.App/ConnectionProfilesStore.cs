using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

namespace Viewer.App;

public static class ConnectionProfilesStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
    };

    private static string GetProfilesPath()
    {
        var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var dir = Path.Combine(appData, "EntheusStreamAnalyticsViewer");
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, "connections.json");
    }

    public static List<ConnectionProfile> Load()
    {
        try
        {
            var path = GetProfilesPath();
            if (!File.Exists(path))
            {
                return new List<ConnectionProfile>();
            }

            var json = File.ReadAllText(path);
            var profiles = JsonSerializer.Deserialize<List<ConnectionProfile>>(json, JsonOptions);
            return profiles ?? new List<ConnectionProfile>();
        }
        catch
        {
            return new List<ConnectionProfile>();
        }
    }

    public static void Save(List<ConnectionProfile> profiles)
    {
        try
        {
            var path = GetProfilesPath();
            var json = JsonSerializer.Serialize(profiles, JsonOptions);
            File.WriteAllText(path, json);
        }
        catch
        {
            // En esta fase de desarrollo ignoramos errores de guardado;
            // más adelante se podría loguear a un archivo separado.
        }
    }
}

