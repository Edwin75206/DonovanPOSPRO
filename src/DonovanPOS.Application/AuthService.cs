using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class AuthService(IDataStore store, IPasswordHasher passwords, UserSession session, IClock clock)
{
    public Task<bool> NeedsSetup() => store.Read(async db => await db.Count<User>() == 0);
    public async Task Setup(string storeName, string username, string name, string password)
    {
        ValidatePassword(password);
        var hash = passwords.Hash(password);
        await store.Write(async db =>
        {
            Amount.Require(await db.Count<User>() == 0, "La instalación ya tiene un administrador.");
            db.Add(new User { Username = Normalize(username), Name = Amount.Text(name, "Nombre", 100), PasswordHash = hash, Role = Role.Administrador });
            db.Add(new StoreSettings { StoreName = Amount.Text(storeName, "Nombre del comercio", 150) });
            Operations.Audit(db, clock, null, "Configuración inicial", "Instalación");
            return true;
        });
    }
    public async Task Login(string username, string password)
    {
        var result = await store.Write(async db =>
        {
            var normalized = Normalize(username);
            var user = (await db.List<User>(x => x.Username == normalized)).SingleOrDefault();
            if (user is null || !user.Active) { passwords.Verify(password, passwords.Hash("timing-only-value")); return (User?)null; }
            if (user.LockedUntil > clock.Now) return null;
            if (!passwords.Verify(password, user.PasswordHash))
            {
                user.FailedAttempts++;
                if (user.FailedAttempts >= 5) { user.LockedUntil = clock.Now.AddMinutes(5); user.FailedAttempts = 0; }
                Operations.Audit(db, clock, user.Id, "Acceso rechazado", user.Username);
                return null;
            }
            user.FailedAttempts = 0; user.LockedUntil = null;
            Operations.Audit(db, clock, user.Id, "Inicio de sesión", user.Username);
            return user;
        });
        Amount.Require(result is not null, "Usuario o contraseña incorrectos. Tras varios intentos, espera 5 minutos.");
        session.SignIn(result!);
    }
    public Task<List<User>> Users() { session.Require(Permission.Users); return store.Read(db => db.List<User>()); }
    public Task<int> SaveUser(int id, string username, string name, Role role, bool active, string? password)
    {
        var actor = session.Require(Permission.Users);
        if (id == 0 || !string.IsNullOrEmpty(password)) ValidatePassword(password ?? "");
        var hash = string.IsNullOrEmpty(password) ? null : passwords.Hash(password);
        return store.Write(async db =>
        {
            var normalized = Normalize(username);
            Amount.Require(!await db.List<User>(x => x.Username == normalized && x.Id != id).ContinueWith(t => t.Result.Any()), "El nombre de usuario ya existe.");
            Amount.Require(id != actor.Id || (active && role == Role.Administrador), "No puedes desactivar tu propia cuenta ni quitarte el rol de administrador.");
            var user = id == 0 ? new User() : await db.Get<User>(id) ?? throw new RuleException("No se encontró el usuario.");
            if (user.Role == Role.Administrador && (!active || role != Role.Administrador))
                Amount.Require((await db.List<User>(x => x.Active && x.Role == Role.Administrador && x.Id != id)).Count > 0, "Debe quedar un administrador activo.");
            user.Username = normalized; user.Name = Amount.Text(name, "Nombre", 100); user.Role = role; user.Active = active;
            if (hash is not null) { user.PasswordHash = hash; user.FailedAttempts = 0; user.LockedUntil = null; }
            if (id == 0) db.Add(user);
            await db.Flush();
            Operations.Audit(db, clock, actor.Id, "Guardar usuario", user.Id.ToString());
            return user.Id;
        });
    }
    private static string Normalize(string username) => Amount.Text(username, "Usuario", 50).ToLowerInvariant();
    private static void ValidatePassword(string password) => Amount.Require(password.Length is >= 10 and <= 128, "La contraseña debe tener entre 10 y 128 caracteres.");
}
