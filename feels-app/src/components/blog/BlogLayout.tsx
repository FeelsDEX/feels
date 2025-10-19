import { format } from 'date-fns'
import Image from 'next/image'
import type { Post } from '@/.velite'
import { proseStyles } from '@/lib/prose-styles'
import { ProcessedContent } from '@/components/content/ProcessedContent'

interface BlogLayoutProps {
  post: Post
}

export function BlogLayout({ post }: BlogLayoutProps) {
  return (
    <article className="container mx-auto px-4 py-12">
      <div className="max-w-3xl mx-auto">
        <header className="mb-6">
          {post.coverImage && (
            <div className="mb-8 -mx-4 sm:mx-0">
              <Image
                src={post.coverImage}
                alt={post.title}
                width={1200}
                height={630}
                className="w-full h-auto rounded-lg"
                priority
              />
            </div>
          )}
          <h1 className="text-[2.25rem] font-bold mb-6 tracking-tight">{post.title}</h1>
          <div className="flex items-center text-base text-muted-foreground">
            {post.author && (
              <>
                <span>{post.author}</span>
                <span className="mx-2">•</span>
              </>
            )}
            <time dateTime={post.date}>
              {format(new Date(post.date), 'd MMMM yyyy')}
            </time>
          </div>
        </header>

        <ProcessedContent 
          className={proseStyles}
          content={post.content}
        />

      </div>
    </article>
  )
}