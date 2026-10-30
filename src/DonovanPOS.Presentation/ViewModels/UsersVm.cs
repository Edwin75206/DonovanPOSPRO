using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed class UsersVm : RecordsVm
{
    private readonly AuthService auth; private readonly Dialogs dialogs;
    public UsersVm(AuthService auth, Dialogs dialogs, UiState ui) : base(ui)
    {
        this.auth = auth; this.dialogs = dialogs; Actions.Add(Action("Agregar usuario", () => Edit(null), true)); RowActions.Add(Action("Editar usuario", () => Edit((User)Selected!.Source!)));
    }
    public override string Title => "Usuarios y permisos";
    public override string Subtitle => "Cada persona trabaja con su propia cuenta.";
    public override string[] Headers => ["Nombre", "Usuario", "Rol", "Estado", "Acceso"];
    protected override async Task Reload()
    {
        var users = await auth.Users(); Summary = "Administrador: acceso completo · Cajero: ventas, cotizaciones y caja; sin descuentos ni devoluciones";
        SetRows(users.Where(x => (x.Name + x.Username).Contains(Search, StringComparison.CurrentCultureIgnoreCase)).Select(x => new RecordRow(x.Id, x.Name, x.Username, x.Role.ToString(), x.Active ? "Activo" : "Desactivado", x.LockedUntil > DateTime.Now ? "Bloqueo temporal" : "Disponible", x)));
    }
    private async Task Edit(User? u)
    {
        u ??= new User(); var current = u;
        if (await dialogs.Form(u.Id == 0 ? "Agregar usuario" : "Editar usuario", "La contraseña debe tener al menos 10 caracteres. Al editar, déjala vacía para conservarla.", [new("name", "Nombre", u.Name), new("username", "Usuario", u.Username), new("role", "Rol", u.Role.ToString(), Dialogs.Options<Role>()), new("active", "Cuenta activa", u.Active ? "Sí" : "No", Dialogs.YesNo), new("password", "Contraseña", Password: true)], async f => { await auth.SaveUser(current.Id, f["username"], f["name"], Enum.Parse<Role>(f["role"]), f.Boolean("active"), f["password"]); })) await Reload();
    }
}
