import sys
import argparse
import random
from rich.console import Console
from rich.markdown import Markdown
from rich.table import Table
from rich.panel import Panel
from scraper import Scraper

console = Console()

def show_welcome():
    welcome_text = """
    [bold cyan]S T O A[/bold cyan]
    [dim]Philosophy & Logic in your Terminal[/dim]
    """
    console.print(Panel(welcome_text, expand=False, border_style="blue"))

def list_fallacies():
    with console.status("[bold green]Fetching fallacies..."):
        fallacies = Scraper.get_fallacies()
    
    if not fallacies:
        console.print("[red]No fallacies found.[/red]")
        return

    table = Table(title="Logical Fallacies")
    table.add_column("Index", justify="right", style="cyan", no_wrap=True)
    table.add_column("Title", style="magenta")
    
    for idx, f in enumerate(fallacies, 1):
        table.add_row(str(idx), f['title'])
    
    console.print(table)
    console.print("\nUse [bold]stoa fallacy <index>[/bold] to read.")

def show_fallacy(index):
    fallacies = Scraper.get_fallacies()
    try:
        idx = int(index) - 1
        if 0 <= idx < len(fallacies):
            f = fallacies[idx]
            with console.status(f"[bold green]Loading {f['title']}..."):
                content = Scraper.get_fallacy_content(f['url'])
            console.print(Panel(f"[bold magenta]Fallacy: {f['title']}[/bold magenta]", expand=False))
            console.print(Markdown(content))
        else:
            console.print("[red]Invalid index.[/red]")
    except ValueError:
        console.print("[red]Please provide a numeric index.[/red]")

def search_sep(query):
    with console.status(f"[bold green]Searching SEP for '{query}'..."):
        entries = Scraper.get_sep_entries()
        results = [e for e in entries if query.lower() in e['title'].lower()]
    
    if not results:
        console.print(f"[yellow]No results found for '{query}'.[/yellow]")
        return

    table = Table(title=f"SEP Results for '{query}'")
    table.add_column("Index", justify="right", style="cyan", no_wrap=True)
    table.add_column("Title", style="magenta")
    
    for idx, e in enumerate(results[:20], 1):
        table.add_row(str(idx), e['title'])
    
    console.print(table)
    if len(results) > 20:
        console.print(f"[dim]... and {len(results)-20} more results.[/dim]")
    
    console.print("\nUse [bold]stoa read \"Title\"[/bold] to read an entry.")

def read_sep(title_query):
    with console.status("[bold green]Finding entry..."):
        entries = Scraper.get_sep_entries()
        match = None
        for e in entries:
            if title_query.lower() == e['title'].lower():
                match = e
                break
        if not match:
            for e in entries:
                if title_query.lower() in e['title'].lower():
                    match = e
                    break
    
    if match:
        with console.status(f"[bold green]Loading {match['title']}..."):
            content = Scraper.get_sep_content(match['url'])
        
        # Header Panel
        console.print("\n")
        console.print(Panel(
            f"[bold magenta]SEP ENCYCLOPEDIA[/bold magenta]\n[bold white]{match['title'].upper()}[/bold white]",
            expand=True,
            border_style="cyan",
            padding=(1, 2)
        ))
        
        # Content with padding
        console.print("\n")
        console.print(Markdown(content))
        console.print("\n" + "─" * console.width + "\n")
    else:
        console.print(f"[red]Could not find SEP entry for '{title_query}'.[/red]")

def random_sep():
    with console.status("[bold green]Picking a random philosophy topic..."):
        entries = Scraper.get_sep_entries()
        if entries:
            entry = random.choice(entries)
            read_sep(entry['title'])
        else:
            console.print("[red]Could not load entries.[/red]")

def main():
    parser = argparse.ArgumentParser(description="Stoa: Philosophy & Logic in Terminal")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("fallacies", help="List all logical fallacies")
    
    fallacy_parser = subparsers.add_parser("fallacy", help="Read a specific fallacy by index")
    fallacy_parser.add_argument("index", help="Index of the fallacy from the list")

    search_parser = subparsers.add_parser("search", help="Search the Stanford Encyclopedia of Philosophy")
    search_parser.add_argument("query", help="Search term")

    read_parser = subparsers.add_parser("read", help="Read a SEP entry by title")
    read_parser.add_argument("title", help="Title or part of the title of the entry")

    subparsers.add_parser("random", help="Read a random entry from SEP")

    args = parser.parse_args()

    if args.command is None:
        show_welcome()
        parser.print_help()
    elif args.command == "fallacies":
        list_fallacies()
    elif args.command == "fallacy":
        show_fallacy(args.index)
    elif args.command == "search":
        search_sep(args.query)
    elif args.command == "read":
        read_sep(args.title)
    elif args.command == "random":
        random_sep()
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
