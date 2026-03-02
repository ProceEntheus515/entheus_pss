using System;
using System.Collections.Generic;
using PerimeterGuard.Configuration;

namespace Viewer.App;

public class ConnectionProfile
{
    public string Id { get; set; } = Guid.NewGuid().ToString();
    public string Name { get; set; } = string.Empty;
    public List<CameraConfig> Cameras { get; set; } = new();
}

