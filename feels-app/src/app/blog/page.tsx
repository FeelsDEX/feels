import { posts } from '@/.velite'
import { BlogCard } from '@/components/blog/BlogCard'

export default function BlogPage() {
  // Filter out drafts and sort by date
  const publishedPosts = posts
    .filter(post => !post.draft)
    .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())

  return (
    <div className="container mx-auto px-4 py-12">
      <div className="max-w-3xl mx-auto">
        <div className="space-y-8">
          {publishedPosts.map((post) => (
            <BlogCard key={post.slug} post={post} />
          ))}
        </div>
      </div>
    </div>
  )
}