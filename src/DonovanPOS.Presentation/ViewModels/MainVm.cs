using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed record NavigationItem(string Key, string Label, string Icon);
public sealed partial class MainVm(AuthService auth, UserSession session, SettingsService settings, Dialogs dialogs, UiState ui, Func<Action<string>, Func<Task>, Dictionary<string, PageVm>> pageFactory) : ObservableObject
{
    public UiState Ui => ui;
    public ObservableCollection<NavigationItem> Navigation { get; } = [];
    [ObservableProperty] private PageVm? currentPage;
    [ObservableProperty] private NavigationItem? selectedNavigation;
    [ObservableProperty] private bool loggedIn;
    [ObservableProperty] private string storeName = "Donovan POS Pro";
    [ObservableProperty] private string userName = "";
    [ObservableProperty] private string roleLabel = "";
    public string DateLabel => DateTime.Now.ToString("dddd, dd 'de' MMMM", Mx.Culture);
    private Dictionary<string, PageVm> pages = [];
    public async Task Initialize() { CurrentPage = new LoginVm(auth, ui, SignedIn); await CurrentPage.Load(); }
    public async Task SignedIn()
    {
        var s = await settings.Get(); StoreName = s.StoreName; UserName = session.Current!.Name; RoleLabel = session.Current.Role.ToString(); ThemeManager.ReducedMotion = s.ReducedMotion; ThemeManager.Apply(s.DarkMode);
        pages = pageFactory(key => _ = NavigateTo(key), SignedOut); Navigation.Clear();
        var all = new[] { new NavigationItem("dashboard", "Inicio", "\uE80F"), new("pos", "Punto de venta", "\uE719"), new("products", "Productos", "\uE7B8"), new("inventory", "Inventario", "\uE7C3"), new("quotes", "Cotizaciones", "\uE8A5"), new("customers", "Clientes y créditos", "\uE716"), new("purchases", "Proveedores y compras", "\uE7BF"), new("reports", "Reportes", "\uE9D9"), new("cash", "Caja", "\uE8C7"), new("users", "Usuarios y permisos", "\uE77B"), new("settings", "Configuración", "\uE713") };
        foreach (var item in all.Where(x => session.Current.Role == Role.Administrador || x.Key is "pos" or "quotes" or "cash")) Navigation.Add(item);
        LoggedIn = true; ui.Notification = ""; SelectedNavigation = Navigation.First(); await NavigateTo(SelectedNavigation.Key);
    }
    partial void OnSelectedNavigationChanged(NavigationItem? value) { if (value is not null && LoggedIn) _ = NavigateTo(value.Key); }
    public async Task NavigateTo(string key)
    {
        if (!pages.TryGetValue(key, out var page)) return;
        try { CurrentPage = page; await page.Load(); } catch (Exception ex) { ui.Notify(ui.Error(ex), true); }
    }
    [RelayCommand] private Task Navigate(string key) => NavigateTo(key);
    [RelayCommand] private void Theme() => ThemeManager.Apply(!ThemeManager.Dark);
    [RelayCommand] private void Dismiss() => ui.Notification = "";
    [RelayCommand] private async Task Logout()
    {
        var hasCart = pages.GetValueOrDefault("pos") is PosVm { IsEmpty: false };
        if (!hasCart || await dialogs.Confirm("Cerrar sesión", "Hay una venta sin cobrar. Sus partidas se descartarán al cerrar sesión.", () => Task.CompletedTask)) await SignedOut();
    }
    private async Task SignedOut() { session.SignOut(); LoggedIn = false; pages.Clear(); Navigation.Clear(); SelectedNavigation = null; ThemeManager.Apply(false); await Initialize(); }
}
