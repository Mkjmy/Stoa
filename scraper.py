import requests
from bs4 import BeautifulSoup
import re

class Scraper:
    @staticmethod
    def get_sep_entries():
        url = "https://plato.stanford.edu/contents.html"
        try:
            response = requests.get(url, timeout=10)
            soup = BeautifulSoup(response.text, 'html.parser')
            content_div = soup.find('div', id='content')
            entries = []
            if content_div:
                for a in content_div.find_all('a'):
                    href = a.get('href', '')
                    if 'entries/' in href:
                        # Ensure relative links are handled
                        if not href.startswith('http'):
                            full_url = f"https://plato.stanford.edu/{href}"
                        else:
                            full_url = href
                        title = a.text.strip()
                        if title:
                            entries.append({'title': title, 'url': full_url})
            
            # De-duplicate
            seen = set()
            unique_entries = []
            for e in entries:
                if e['url'] not in seen:
                    unique_entries.append(e)
                    seen.add(e['url'])
            return unique_entries
        except Exception as e:
            return []

    @staticmethod
    def get_sep_content(url):
        try:
            response = requests.get(url, timeout=10)
            soup = BeautifulSoup(response.text, 'html.parser')
            
            # The main content is usually in #main-content or #content
            main_content = soup.find('div', id='main-content') or soup.find('div', id='content')
            
            if main_content:
                # Deep cleaning of boilerplate
                # Remove navigation, headers, and footer-like sections within the content
                for extra in main_content.find_all(['nav', 'script', 'style']):
                    extra.decompose()
                
                # Specifically targeting SEP boilerplate IDs and classes
                for extra_id in ['article-nav', 'toc', 'bibliography', 'academic-tools', 'other-internet-resources', 'related-entries']:
                    found = main_content.find('div', id=extra_id) or main_content.find('section', id=extra_id)
                    if found:
                        found.decompose()

                # Remove the navigation text blocks we saw in the output
                for h2 in main_content.find_all('h2'):
                    if h2.text.strip() in ['Entry Navigation', 'Entry Contents', 'Bibliography', 'Academic Tools', 'Friends PDF Preview', 'Author and Citation Info', 'Back to Top']:
                        # Remove the header and its following siblings until next section? 
                        # Actually, just removing the header helps, but we want the sections below it too.
                        pass

                text = main_content.get_text(separator='\n\n')
                
                # Regex to clean up repetitive navigation strings that get caught in text
                boilerplate_patterns = [
                    r"Entry Navigation",
                    r"Entry Contents",
                    r"Bibliography",
                    r"Academic Tools",
                    r"Friends PDF Preview",
                    r"Author and Citation Info",
                    r"Back to Top",
                    r"Other Internet Resources",
                    r"Related Entries"
                ]
                for pattern in boilerplate_patterns:
                    text = re.sub(rf"^\s*{pattern}\s*$", "", text, flags=re.MULTILINE)

                return text.strip()
            return "Could not find content."
        except Exception as e:
            return str(e)

    @staticmethod
    def get_fallacies():
        url = "https://yourlogicalfallacyis.com/"
        try:
            response = requests.get(url, timeout=10)
            soup = BeautifulSoup(response.text, 'html.parser')
            fallacies = []
            for a in soup.find_all('a', class_='square-button'):
                href = a.get('href', '')
                title = a.get('title') or a.text.strip()
                if href and title:
                    if not href.startswith('http'):
                        full_url = f"https://yourlogicalfallacyis.com{href}"
                    else:
                        full_url = href
                    fallacies.append({'title': title.capitalize(), 'url': full_url})
            
            return fallacies
        except Exception as e:
            return []

    @staticmethod
    def get_fallacy_content(url):
        try:
            response = requests.get(url, timeout=10)
            soup = BeautifulSoup(response.text, 'html.parser')
            # Look for definition and examples
            # Usually in a main content area
            content = []
            
            title_tag = soup.find('h1')
            if title_tag:
                content.append(f"# {title_tag.text.strip()}\n")
            
            # Definition and Example are often in specific divs or classes
            # Let's try to get the primary text blocks
            main_sections = soup.find_all(['p', 'h2', 'h3'])
            for section in main_sections:
                text = section.text.strip()
                if text:
                    content.append(text)
            
            return "\n\n".join(content)
        except Exception as e:
            return str(e)
