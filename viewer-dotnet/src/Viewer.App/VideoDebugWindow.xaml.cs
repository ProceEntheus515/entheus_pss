using System.Windows;
using LibVLCSharp.Shared;
using LibVLCSharp.WPF;

namespace Viewer.App;

public partial class VideoDebugWindow : Window
{
    public VideoDebugWindow(MediaPlayer mediaPlayer)
    {
        InitializeComponent();
        DebugVideoView.MediaPlayer = mediaPlayer;
    }
}

