import Link from 'next/link'
import { format } from 'date-fns'
import Image from 'next/image'
import type { Post } from '@/.velite'

interface BlogCardProps {
  post: Post
}

export function BlogCard({ post }: BlogCardProps) {
  return (
    <article className="border rounded-lg hover:shadow-lg hover:border-primary transition-all cursor-pointer">
      <Link href={post.permalink} className="flex p-6 gap-6">
        {post.coverImage && (
          <div className="relative w-48 h-32 flex-shrink-0">
            <Image
              src={post.coverImage}
              alt={post.title}
              fill
              className="object-cover rounded-md"
              sizes="192px"
            />
          </div>
        )}
        <div className="flex-1 space-y-3">
          <div className="flex items-center text-sm text-muted-foreground">
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
          <h2 className="text-2xl font-semibold">{post.title}</h2>
          <p className="text-muted-foreground">{post.description}</p>
        </div>
      </Link>
    </article>
  )
}