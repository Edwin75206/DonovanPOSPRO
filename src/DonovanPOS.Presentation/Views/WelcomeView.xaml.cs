using System.Windows;
using System.Windows.Controls;
using DonovanPOS.Presentation.ViewModels;
namespace DonovanPOS.Presentation.Views;
public partial class WelcomeView : UserControl
{
    public WelcomeView() => InitializeComponent();
    private void PasswordChanged(object sender, RoutedEventArgs e) { if (DataContext is LoginVm vm) { vm.Password = Password.Password; vm.PasswordConfirmation = Confirmation.Password; } }
}
