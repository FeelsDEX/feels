import { defineConfig, s } from 'velite'
import rehypePrettyCode from 'rehype-pretty-code'
import rehypeSlug from 'rehype-slug'
import remarkMath from 'remark-math'
import rehypeKatex from 'rehype-katex'

export default defineConfig({
  root: '.',
  output: {
    data: '.velite',
    assets: 'public/static',
    base: '/static/',
    name: '[name]-[hash:6].[ext]',
    clean: true
  },
  collections: {
    posts: {
      name: 'Post',
      pattern: 'content/blog/*.md',
      schema: s
        .object({
          title: s.string().max(99),
          description: s.string(),
          date: s.isodate(),
          author: s.string(),
          tags: s.array(s.string()).optional(),
          coverImage: s.string().optional(),
          draft: s.boolean().default(false),
          slug: s.string().optional(), // Custom slug field
          metadata: s.metadata(),
          excerpt: s.excerpt(),
          content: s.markdown()
        })
        .transform((data, { meta }) => {
          // Use custom slug if provided, otherwise use filename
          const filename = (meta as any).path.split('/').pop() || ''
          const defaultSlug = filename.replace(/\.md$/, '')
          const slug = data.slug || defaultSlug
          
          return {
            ...data,
            slug,
            permalink: `/blog/${slug}`
          }
        })
    },
    docs: {
      name: 'Doc',
      pattern: 'content/docs/*.md',
      schema: s
        .object({
          title: s.string().max(99),
          description: s.string(),
          category: s.string(),
          order: s.number().default(999),
          draft: s.boolean().default(false),
          searchable: s.boolean().default(true),
          slug: s.string().optional(), // Custom slug field
          metadata: s.metadata(),
          content: s.markdown()
        })
        .transform((data, { meta }) => {
          // Use custom slug if provided, otherwise use filename
          const filename = (meta as any).path.split('/').pop() || ''
          const defaultSlug = filename.replace(/\.md$/, '')
          const slug = data.slug || defaultSlug
          
          return {
            ...data,
            slug,
            permalink: slug === '001-introduction' || slug === 'introduction' ? '/docs' : `/docs/${slug}`
          }
        })
    }
  },
  markdown: {
    remarkPlugins: [remarkMath],
    rehypePlugins: [
      rehypeSlug,
      rehypeKatex,
      [
        rehypePrettyCode,
        {
          theme: 'github-light-default',
          keepBackground: false,
          defaultLang: 'plaintext',
          transformers: [],
          onVisitLine(node: any) {
            if (node.children.length === 0) {
              node.children = [{ type: 'text', value: ' ' }]
            }
          },
          onVisitHighlightedLine(node: any) {
            node.properties.className = node.properties.className || []
            node.properties.className.push('highlighted')
          }
        }
      ]
    ]
  }
})