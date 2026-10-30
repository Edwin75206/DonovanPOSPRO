using System.Windows.Controls;
using DonovanPOS.Presentation.ViewModels;
namespace DonovanPOS.Presentation.Views;
public partial class RegisterView : UserControl
{
    public RegisterView() { InitializeComponent(); Loaded += (_, _) => { if (DataContext is PosVm vm) vm.FocusSearch = () => { SearchBox.Focus(); SearchBox.SelectAll(); }; SearchBox.Focus(); }; }
}
