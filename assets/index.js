document.body.addEventListener('click', e => {
        if (e.target.tagName === 'A' && e.target.getAttribute('href').startsWith('?')) {
                const link = new URLSearchParams(e.target.getAttribute('href'))
                const current = new URLSearchParams(location.search)
                const sortBy = link.get('sort_by') ?? current.get('sort_by')
                const asc = link.get('sort_by') ? link.get('asc') : current.get('asc')
                const filter = [...link.entries(), ...current.entries()].find(([key, value]) => key !== 'sort_by' && key !== 'asc')
                const combined = new URLSearchParams()
                if (sortBy) combined.set('sort_by', sortBy)
                if (asc) combined.set('asc', asc)
                if (filter) combined.set(filter[0], filter[1])
                const target = new URL(e.target.href)
                target.search = combined
                e.preventDefault()
                location.href = target
        }
})
