using DonovanPOS.Domain;
using System.Collections;
using System.Data;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Media;
using DonovanPOS.Application;
using Microsoft.Win32;
namespace DonovanPOS.Presentation.Common;

public sealed record Choice(string Value, string Label) { public override string ToString() => Label; }
public sealed record Field(string Key, string Label, string Value = "", IReadOnlyList<Choice>? Options = null, bool Password = false, bool Multiline = false, bool ReadOnly = false);
public sealed class FormValues(Dictionary<string, string> values)
{
    public string this[string key] => values.GetValueOrDefault(key, "");
    public decimal Number(string key) => Mx.Number(this[key]);
    public int Integer(string key) => int.Parse(this[key], Mx.Culture);
    public bool Boolean(string key) => this[key] == "Sí";
}
public sealed partial class Dialogs(UiState ui)
{
    public static Choice[] YesNo => [new("No", "No"), new("Sí", "Sí")];
    public static Choice[] Options<T>() where T : Enum => Enum.GetNames(typeof(T)).Select(x => new Choice(x, x)).ToArray();
    public Task<bool> Form(string title, string description, IReadOnlyList<Field> fields, Func<FormValues, Task> save, string action = "Guardar", Func<FormValues, string>? liveSummary = null)
    {
        var completion = new TaskCompletionSource<bool>();
        var window = new Window { Title = title, Owner = System.Windows.Application.Current.MainWindow, Width = fields.Count > 5 ? 760 : 510, Height = Math.Min(740, Math.Max(350, 230 + Math.Ceiling(fields.Count / (fields.Count > 5 ? 2d : 1)) * 84)), WindowStartupLocation = WindowStartupLocation.CenterOwner, ResizeMode = ResizeMode.CanResize, MinWidth = 420, MinHeight = 300 };
        window.MaxHeight = SystemParameters.WorkArea.Height; window.MaxWidth = SystemParameters.WorkArea.Width;
        var root = new DockPanel { Margin = new Thickness(26) }; window.Content = root;
        var heading = new StackPanel { Margin = new Thickness(0, 0, 0, 20) }; heading.Children.Add(new TextBlock { Text = title, FontSize = 24, FontWeight = FontWeights.SemiBold }); heading.Children.Add(new TextBlock { Text = description, Margin = new Thickness(0, 6, 0, 0), Style = (Style)window.FindResource("MutedText") }); DockPanel.SetDock(heading, Dock.Top); root.Children.Add(heading);
        var bottom = new StackPanel { Margin = new Thickness(0, 15, 0, 0) }; var error = new TextBlock { TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 0, 0, 10) }; error.SetResourceReference(TextBlock.ForegroundProperty, "Danger"); bottom.Children.Add(error);
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right }; var cancel = new Button { Content = "Volver", Margin = new Thickness(0, 0, 10, 0), IsCancel = true }; cancel.Click += (_, _) => window.Close();
        var submit = new Button { Content = action, IsDefault = true, Style = (Style)window.FindResource("PrimaryButton") }; buttons.Children.Add(cancel); buttons.Children.Add(submit); bottom.Children.Add(buttons); DockPanel.SetDock(bottom, Dock.Bottom); root.Children.Add(bottom);
        var grid = new Grid(); var count = fields.Count > 5 ? 2 : 1; for (var i = 0; i < count; i++) grid.ColumnDefinitions.Add(new());
        var readers = new Dictionary<string, Func<string>>();
        for (var i = 0; i < fields.Count; i++)
        {
            if (i % count == 0) grid.RowDefinitions.Add(new() { Height = GridLength.Auto }); var f = fields[i];
            var panel = new StackPanel { Margin = new Thickness(0, 0, i % count == 0 && count == 2 ? 16 : 0, 14) }; panel.Children.Add(new TextBlock { Text = f.Label, FontWeight = FontWeights.SemiBold, FontSize = 12, Margin = new Thickness(0, 0, 0, 6) });
            FrameworkElement control;
            if (f.Options is not null)
            {
                var combo = new ComboBox { ItemsSource = f.Options, SelectedValuePath = "Value", DisplayMemberPath = "Label", SelectedValue = f.Value, IsEnabled = !f.ReadOnly }; control = combo; readers[f.Key] = () => combo.SelectedValue?.ToString() ?? "";
            }
            else if (f.Password)
            {
                var box = new PasswordBox(); control = box; readers[f.Key] = () => box.Password;
            }
            else
            {
                var box = new TextBox { Text = f.Value, IsReadOnly = f.ReadOnly, AcceptsReturn = f.Multiline, TextWrapping = f.Multiline ? TextWrapping.Wrap : TextWrapping.NoWrap, MinHeight = f.Multiline ? 72 : 36 }; control = box; readers[f.Key] = () => box.Text;
            }
            System.Windows.Automation.AutomationProperties.SetName(control, f.Label); panel.Children.Add(control); Grid.SetColumn(panel, i % count); Grid.SetRow(panel, i / count); grid.Children.Add(panel);
        }
        root.Children.Add(new ScrollViewer { Content = grid, VerticalScrollBarVisibility = ScrollBarVisibility.Auto });
        if (liveSummary is not null)
        {
            var summary = new TextBlock { FontSize = 18, FontWeight = FontWeights.SemiBold, Margin = new Thickness(0, 0, 0, 12) }; bottom.Children.Insert(0, summary);
            void Update() { try { summary.Text = liveSummary(new FormValues(readers.ToDictionary(x => x.Key, x => x.Value()))); } catch { summary.Text = "Revisa los importes"; } }
            grid.AddHandler(TextBox.TextChangedEvent, new TextChangedEventHandler((_, _) => Update())); grid.AddHandler(ComboBox.SelectionChangedEvent, new SelectionChangedEventHandler((_, _) => Update())); Update();
        }
        var working = false;
        submit.Click += async (_, _) =>
        {
            if (working) return; working = true; submit.IsEnabled = false; cancel.IsEnabled = false; error.Text = "Guardando…";
            try { await save(new FormValues(readers.ToDictionary(x => x.Key, x => x.Value()))); completion.TrySetResult(true); window.Close(); }
            catch (Exception ex) { error.Text = ui.Error(ex); }
            finally { working = false; submit.IsEnabled = true; cancel.IsEnabled = true; }
        };
        window.Closing += (_, e) => { if (working && !completion.Task.IsCompleted) e.Cancel = true; };
        window.Closed += (_, _) => completion.TrySetResult(false);
        window.Loaded += (_, _) => { ThemeManager.Enter(root); grid.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.First)); };
        window.ShowDialog(); return completion.Task;
    }
    public Task<bool> Confirm(string title, string description, Func<Task> action) => Form(title, description, [], _ => action(), "Confirmar");
    public string? SavePath(string filename, string extension)
    {
        var dialog = new SaveFileDialog { FileName = filename, DefaultExt = extension, Filter = extension == ".pdf" ? "Documento PDF|*.pdf" : "Libro de Excel|*.xlsx", AddExtension = true };
        return dialog.ShowDialog(System.Windows.Application.Current.MainWindow) == true ? dialog.FileName : null;
    }
    public string? OpenPath(string filter) { var dialog = new OpenFileDialog { Filter = filter }; return dialog.ShowDialog(System.Windows.Application.Current.MainWindow) == true ? dialog.FileName : null; }
    public string? Folder() { var dialog = new OpenFolderDialog { Title = "Seleccionar carpeta" }; return dialog.ShowDialog(System.Windows.Application.Current.MainWindow) == true ? dialog.FolderName : null; }
    public void Table(TableData data)
    {
        var table = new DataTable(); foreach (var header in data.Headers) table.Columns.Add(header); foreach (var row in data.Rows) table.Rows.Add(row.Cast<object>().ToArray());
        var window = new Window { Title = data.Title, Owner = System.Windows.Application.Current.MainWindow, Width = 920, Height = 580, WindowStartupLocation = WindowStartupLocation.CenterOwner, MaxWidth = SystemParameters.WorkArea.Width, MaxHeight = SystemParameters.WorkArea.Height };
        var grid = new DataGrid { ItemsSource = table.DefaultView, AutoGenerateColumns = true, Margin = new Thickness(20) }; window.Content = grid; window.ShowDialog();
    }
}

