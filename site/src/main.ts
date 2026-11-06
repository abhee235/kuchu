// Every page's styles; on the home page, the live pet too (its own chunk, so the docs load none of it).
import './site.css'

if (document.querySelector('[data-pet]')) void import('./demo')

// "Copy" beside a command: the command goes to the clipboard, and the button says so for a moment.
document.addEventListener('click', (e) => {
  const button = (e.target as HTMLElement).closest<HTMLButtonElement>('[data-copy]')
  if (!button?.dataset.copy) return
  void navigator.clipboard.writeText(button.dataset.copy).then(() => {
    button.textContent = 'Copied'
    setTimeout(() => (button.textContent = 'Copy'), 1600)
  })
})

// On a narrow screen the docs' menu is one scrolling row: it starts at the page you are on.
const here = document.querySelector<HTMLElement>('.docs-nav [aria-current]')
const row = here?.closest('ul')
if (here && row && row.scrollWidth > row.clientWidth) row.scrollLeft = here.offsetLeft - 12
