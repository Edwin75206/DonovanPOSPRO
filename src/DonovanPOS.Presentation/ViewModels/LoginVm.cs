using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Presentation.Common;
using DonovanPOS.Domain;
using System.Printing;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class LoginVm(AuthService auth, Dialogs dialogs, UiState ui, Func<Task> signedIn) : PageVm(ui)
{
    public override string Title => Setup ? new[] { "Bienvenido a Donovan", "Tu cuenta administradora", "A tu manera", "Listo para empezar" }[SetupStep] : "Bienvenido de nuevo";
    [ObservableProperty] private bool setup;
    [ObservableProperty] private string username = "";
    [ObservableProperty] private string name = "";
    [ObservableProperty] private string storeName = "";
    [ObservableProperty] private string error = "";
    [ObservableProperty] private int setupStep;
    [ObservableProperty] private string taxPercent = "16";
    public StoreSettings Preferences { get; } = new() { BackupDirectory = System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "DonovanSystems", "DonovanPOS", "respaldos") };
    public string[] Printers { get; private set; } = [""];
    public int[] TicketWidths { get; } = [58, 80];
    public bool CommerceStep => Setup && SetupStep == 0;
    public bool AccountStep => !Setup || SetupStep == 1;
    public bool PreferencesStep => Setup && SetupStep == 2;
    public bool BackupStep => Setup && SetupStep == 3;
    public bool CanBack => Setup && SetupStep > 0;
    public string StepLabel => Setup ? $"PASO {SetupStep + 1} DE 4 · CONFIGURACIÓN INICIAL" : "TU FERRETERÍA, ORGANIZADA";
    public string Description => !Setup ? "Accede con tu cuenta para comenzar." : new[] { "Dale identidad a tu ferretería. Podrás ajustar estos datos después.", "Esta cuenta administrará el negocio. Elige tus propias credenciales.", "Moneda MXN y español de México. Precios con IVA incluido.", "Elige tu impresora y dónde conservar las copias de seguridad." }[SetupStep];
    public string Password { private get; set; } = "";
    public string PasswordConfirmation { private get; set; } = "";
    public string ActionText => Setup ? SetupStep == 3 ? "Crear mi ferretería" : "Continuar" : "Entrar a mi ferretería";
    public override async Task Load() { Setup = await auth.NeedsSetup(); ChangedStep(); if (Setup) try { using var server = new LocalPrintServer(); Printers = new[] { "" }.Concat(server.GetPrintQueues().Select(x => x.Name)).ToArray(); OnPropertyChanged(nameof(Printers)); } catch (PrintSystemException) { } }
    partial void OnSetupStepChanged(int value) => ChangedStep();
    private void ChangedStep() { foreach (var name in new[] { nameof(Title), nameof(ActionText), nameof(CommerceStep), nameof(AccountStep), nameof(PreferencesStep), nameof(BackupStep), nameof(CanBack), nameof(StepLabel), nameof(Description) }) OnPropertyChanged(name); }
    [RelayCommand] private void Back() { if (CanBack) { SetupStep--; Error = ""; } }
    [RelayCommand] private void BackupFolder() { var folder = dialogs.Folder(); if (folder is not null) { Preferences.BackupDirectory = folder; OnPropertyChanged(nameof(Preferences)); } }
    [RelayCommand] private async Task Submit()
    {
        if (Busy) return; Busy = true; Error = "";
        try
        {
            if (Setup)
            {
                if (SetupStep == 0) { Amount.Text(StoreName, "Nombre de la ferretería", 150); SetupStep++; return; }
                if (SetupStep == 1) { Amount.Text(Name, "Tu nombre", 100); Amount.Text(Username, "Usuario", 50); Amount.Require(Password.Length is >= 10 and <= 128, "Usa entre 10 y 128 caracteres para tu contraseña."); Amount.Require(Password == PasswordConfirmation, "Las contraseñas no coinciden."); SetupStep++; return; }
                if (SetupStep == 2) { var tax = Mx.Number(TaxPercent); Amount.Require(tax is >= 0 and <= 100, "Usa un IVA entre 0 y 100 %."); Preferences.DefaultTaxBasisPoints = checked((int)(tax * 100)); SetupStep++; return; }
                DonovanPOS.Domain.Amount.Require(Password == PasswordConfirmation, "Las contraseñas no coinciden.");
                await auth.Setup(StoreName, Username, Name, Password, Preferences);
            }
            await auth.Login(Username, Password); Password = ""; PasswordConfirmation = ""; await signedIn();
        }
        catch (Exception ex) { Error = Ui.Error(ex); }
        finally { Busy = false; }
    }
}
