using System.Windows.Controls;
using System.Windows.Input;
using DonovanPOS.Presentation.ViewModels;
namespace DonovanPOS.Presentation.Views;
public partial class PosView : UserControl
{
    public PosView() { InitializeComponent(); Loaded += (_, _) => { if (DataContext is PosVm vm) vm.FocusSearch = () => { SearchBox.Focus(); SearchBox.SelectAll(); }; SearchBox.Focus(); }; }
    private async void ProductDoubleClick(object sender, MouseButtonEventArgs e) { if (DataContext is PosVm vm && vm.SelectedProduct is not null) await vm.AddCommand.ExecuteAsync(null); }
}
