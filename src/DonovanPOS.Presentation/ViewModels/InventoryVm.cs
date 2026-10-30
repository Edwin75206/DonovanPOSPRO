using DonovanPOS.Application;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;
public sealed class InventoryVm(CatalogService catalog, ImportService import, SettingsService settings, IDocumentService documents, Dialogs dialogs, UiState ui) : CatalogVm(catalog, import, settings, documents, dialogs, ui, true);
