using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;
public partial class CatalogVm
{
    [RelayCommand] private Task Labels() => Run(async () =>
    {
        Amount.Require(Selected is not null, "Selecciona un producto para generar sus etiquetas."); var p = Selected!;
        await dialogs.Form("Etiquetas con código de barras", p.Name + ". PDF en hoja A4, 16 etiquetas por página. Imprime a tamaño real, sin ajustar.", [new("copies", "Número de etiquetas", "16")], async f =>
        {
            var copies = f.Integer("copies"); Amount.Require(copies is >= 1 and <= 500, "Usa entre 1 y 500 etiquetas."); var file = dialogs.SavePath("Etiquetas-" + p.Sku, ".pdf"); if (file is null) return;
            await documents.Labels(file, await settings.Get(), [new(p.Barcode ?? p.Sku, p.Name, p.PriceCents, copies)]); Mx.OpenFile(file);
        }, "Crear PDF");
    });
}
