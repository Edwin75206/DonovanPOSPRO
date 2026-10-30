using System.Windows;
using DonovanPOS.Presentation.Common;
using DonovanPOS.Presentation.ViewModels;
namespace DonovanPOS.Presentation;

public partial class MainWindow : Window
{
    public MainWindow(MainVm vm)
    {
        InitializeComponent(); DataContext = vm;
        Width = Math.Min(1360, SystemParameters.WorkArea.Width); Height = Math.Min(860, SystemParameters.WorkArea.Height);
        SizeChanged += (_, _) => { var small = ActualWidth < 1100; vm.Compact = small; UserDetails.Visibility = small ? Visibility.Collapsed : Visibility.Visible; LogoutButton.Content = small ? "↪" : "Cerrar sesión"; TopBarRow.Height = new GridLength(small ? 48 : 62); SidebarColumn.Width = new GridLength(small ? 80 : 218); SidebarFrame.Margin = new Thickness(small ? 6 : 14, 22, small ? 6 : 14, 16); BrandLabel.Visibility = BrandSub.Visibility = small ? Visibility.Collapsed : Visibility.Visible; PageHost.Margin = new Thickness(small ? 16 : 24, 18, small ? 16 : 24, 12); };
        vm.PropertyChanged += (_, e) => { if (e.PropertyName == nameof(MainVm.CurrentPage)) ThemeManager.Enter(PageHost); };
        var allowedClose = false; var checkingClose = false;
        Closing += async (_, e) =>
        {
            if (allowedClose) return; e.Cancel = true; if (checkingClose) return; checkingClose = true;
            try { if (await vm.CanClose()) { allowedClose = true; _ = Dispatcher.BeginInvoke(Close); } }
            finally { checkingClose = false; }
        };
    }
}


