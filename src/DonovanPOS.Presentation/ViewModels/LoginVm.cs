using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class LoginVm(AuthService auth, UiState ui, Func<Task> signedIn) : PageVm(ui)
{
    public override string Title => Setup ? "Preparar tu ferretería" : "Bienvenido de nuevo";
    [ObservableProperty] private bool setup;
    [ObservableProperty] private string username = "";
    [ObservableProperty] private string name = "";
    [ObservableProperty] private string storeName = "";
    [ObservableProperty] private string error = "";
    public string Password { private get; set; } = "";
    public string PasswordConfirmation { private get; set; } = "";
    public string ActionText => Setup ? "Crear administrador y continuar" : "Entrar a mi ferretería";
    public override async Task Load() { Setup = await auth.NeedsSetup(); OnPropertyChanged(nameof(Title)); OnPropertyChanged(nameof(ActionText)); }
    [RelayCommand] private async Task Submit()
    {
        if (Busy) return; Busy = true; Error = "";
        try
        {
            if (Setup)
            {
                DonovanPOS.Domain.Amount.Require(Password == PasswordConfirmation, "Las contraseñas no coinciden.");
                await auth.Setup(StoreName, Username, Name, Password);
            }
            await auth.Login(Username, Password); Password = ""; PasswordConfirmation = ""; await signedIn();
        }
        catch (Exception ex) { Error = Ui.Error(ex); }
        finally { Busy = false; }
    }
}
