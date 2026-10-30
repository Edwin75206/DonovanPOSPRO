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
        SizeChanged += (_, _) => { var small = ActualWidth < 1100; SidebarColumn.Width = new GridLength(small ? 64 : 218); BrandLabel.Visibility = BrandSub.Visibility = small ? Visibility.Collapsed : Visibility.Visible; PageHost.Margin = new Thickness(small ? 16 : 24, 18, small ? 16 : 24, 12); };
        vm.PropertyChanged += (_, e) => { if (e.PropertyName == nameof(MainVm.CurrentPage)) ThemeManager.Enter(PageHost); };
    }
}
