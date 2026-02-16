<script setup lang="ts">
import Header from './components/Header.vue';
import HeroSection from './components/HeroSection.vue';
import EditorSection from './components/EditorSection.vue';
import FeaturesSection from './components/FeaturesSection.vue';
import FeatureCard from './components/FeatureCard.vue';
import CodeExamplesSection from './components/CodeExamplesSection.vue';
import InstallSection from './components/InstallSection.vue';
import Footer from './components/Footer.vue';
import {
  ActivityIcon,
  UploadIcon,
  MonitorIcon,
  GlobeIcon,
  LockIcon,
  CodeIcon,
} from './components/icons';
</script>

<template>
  <Header />
  <main>
    <HeroSection />
    <EditorSection />
    <FeaturesSection>
      <FeatureCard
        :icon="ActivityIcon"
        :index="0"
        title="Async by Design"
        description="Built on tokio and russh for high-performance asynchronous SSH operations"
      />
      <FeatureCard
        :icon="UploadIcon"
        :index="1"
        title="SCP File Transfers"
        description="Secure file upload and download capabilities with progress tracking"
      />
      <FeatureCard
        :icon="MonitorIcon"
        :index="2"
        title="Interactive PTY"
        description="Full pseudo-terminal support for interactive sessions and command-line interfaces"
      />
      <FeatureCard
        :icon="GlobeIcon"
        :index="3"
        title="IPv6 Link-Local"
        description="Full support for IPv6 including link-local addresses and zone identifiers"
      />
      <FeatureCard
        :icon="LockIcon"
        :index="4"
        title="Flexible Auth"
        description="Multiple authentication methods including password, key-based, and agent forwarding"
      />
      <FeatureCard
        :icon="CodeIcon"
        :index="5"
        title="CLI Tools Included"
        description="Command-line utilities for quick SSH operations and file transfers"
      />
    </FeaturesSection>
    <CodeExamplesSection />
    <InstallSection />
  </main>
  <Footer />
</template>

<style>
/* Ensure app content appears above the grid overlay */
#app {
  position: relative;
  z-index: 1;
}
</style>
