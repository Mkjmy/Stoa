from textual.app import App, ComposeResult
from textual.widgets import Header, Footer, ListView, ListItem, Label, Input, TabbedContent, TabPane, Markdown, Static
from textual.containers import Vertical, Horizontal, Container
from textual.binding import Binding
from scraper import Scraper
import asyncio

class StoaApp(App):
    TITLE = "S T O A"
    SUB_TITLE = "Philosophy & Logic in Terminal"
    CSS_PATH = "styles.tcss"

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("r", "refresh", "Refresh List", show=True),
        Binding("ctrl+s", "search", "Focus Search", show=True),
    ]

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with TabbedContent():
            with TabPane("SEP (Stanford Encyclopedia)", id="sep_tab"):
                yield Input(placeholder="Search SEP...", id="sep_search")
                with Horizontal():
                    yield ListView(id="sep_list")
                    with Vertical(id="content_area"):
                        yield Markdown(id="sep_content")
            
            with TabPane("Logical Fallacies", id="fallacy_tab"):
                with Horizontal():
                    yield ListView(id="fallacy_list")
                    with Vertical(id="content_area_fallacy"):
                        yield Markdown(id="fallacy_content")
        yield Footer()

    def on_mount(self) -> None:
        self.run_worker(self.load_data())

    async def load_data(self):
        self.update_status("Loading fallacies...")
        fallacies = await asyncio.to_thread(Scraper.get_fallacies)
        fallacy_list = self.query_one("#fallacy_list", ListView)
        for f in fallacies:
            fallacy_list.append(ListItem(Label(f['title']), name=f['url']))
        
        self.update_status("Loading SEP Index...")
        sep_entries = await asyncio.to_thread(Scraper.get_sep_entries)
        self.all_sep_entries = sep_entries
        
        sep_list = self.query_one("#sep_list", ListView)
        for entry in sep_entries[:100]:
            sep_list.append(ListItem(Label(entry['title']), name=entry['url']))
        
        self.update_status("Ready.")

    def update_status(self, message: str):
        self.sub_title = message

    async def on_list_view_selected(self, event: ListView.Selected):
        url = event.item.name
        list_id = event.list_view.id
        
        if list_id == "sep_list":
            content_widget = self.query_one("#sep_content", Markdown)
            content_widget.update("Loading content...")
            content = await asyncio.to_thread(Scraper.get_sep_content, url)
            content_widget.update(content)
        elif list_id == "fallacy_list":
            content_widget = self.query_one("#fallacy_content", Markdown)
            content_widget.update("Loading content...")
            content = await asyncio.to_thread(Scraper.get_fallacy_content, url)
            content_widget.update(content)

    def on_input_changed(self, event: Input.Changed):
        if event.input.id == "sep_search":
            search_query = event.value.lower()
            sep_list = self.query_one("#sep_list", ListView)
            sep_list.clear()
            
            # Filter cached entries
            count = 0
            for entry in self.all_sep_entries:
                if search_query in entry['title'].lower():
                    sep_list.append(ListItem(Label(entry['title']), name=entry['url']))
                    count += 1
                if count >= 100: # Limit display for performance
                    break

    def action_search(self):
        self.query_one("#sep_search").focus()

if __name__ == "__main__":
    app = StoaApp()
    app.run()
