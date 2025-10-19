import { notFound } from 'next/navigation'
import { posts } from '@/.velite'
import { BlogLayout } from '@/components/blog/BlogLayout'

interface BlogPostPageProps {
  params: Promise<{
    slug: string
  }>
}

export async function generateStaticParams() {
  return posts.filter(post => !post.draft).map((post) => ({
    slug: post.slug,
  }))
}

export default async function BlogPostPage({ params }: BlogPostPageProps) {
  const { slug } = await params
  const post = posts.find((p) => p.slug === slug)
  
  if (!post || post.draft) {
    notFound()
  }

  return <BlogLayout post={post} />
}