using System.Security.Cryptography;
using DonovanPOS.Application;
namespace DonovanPOS.Infrastructure.Security;

public sealed class PasswordHasher : IPasswordHasher
{
    private const int Iterations = 600_000;
    public string Hash(string password)
    {
        var salt = RandomNumberGenerator.GetBytes(16);
        var hash = Rfc2898DeriveBytes.Pbkdf2(password, salt, Iterations, HashAlgorithmName.SHA256, 32);
        return $"pbkdf2-sha256${Iterations}${Convert.ToBase64String(salt)}${Convert.ToBase64String(hash)}";
    }
    public bool Verify(string password, string encoded)
    {
        try
        {
            var parts = encoded.Split('$');
            if (parts.Length != 4 || parts[0] != "pbkdf2-sha256" || !int.TryParse(parts[1], out var rounds) || rounds < 100_000 || rounds > 2_000_000) return false;
            var actual = Rfc2898DeriveBytes.Pbkdf2(password, Convert.FromBase64String(parts[2]), rounds, HashAlgorithmName.SHA256, 32);
            return CryptographicOperations.FixedTimeEquals(actual, Convert.FromBase64String(parts[3]));
        }
        catch (FormatException) { return false; }
    }
}
