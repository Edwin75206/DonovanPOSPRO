using System.Windows.Controls;
using DonovanPOS.Presentation.ViewModels;
namespace DonovanPOS.Presentation.Views;
public partial class RecordsView : UserControl
{
    public RecordsView() { InitializeComponent(); DataContextChanged += (_, _) => { if (DataContext is RecordsVm vm) for (var i = 0; i < Table.Columns.Count; i++) Table.Columns[i].Header = vm.Headers[i]; }; }
}
